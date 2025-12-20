//! LanceDB storage layer for the document corpus
//!
//! Provides vector storage and retrieval using LanceDB backed by object storage.

use anyhow::{anyhow, Result};
use std::sync::Arc;

use arrow_array::{
    Array, ArrayRef, FixedSizeListArray, Int32Array, Int64Array, LargeStringArray,
    ListArray, RecordBatch, RecordBatchReader, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::query::{ExecutableQuery, QueryBase};

use crate::document::{Document, DocumentMetadata};
use crate::search::{SearchFilters, SearchResult, MatchType};

/// Corpus storage backed by LanceDB on object storage
pub struct CorpusStorage {
    db: Arc<lancedb::Connection>,
    table_name: String,
}

impl CorpusStorage {
    /// Connect to LanceDB backed by object storage
    ///
    /// # Arguments
    /// * `uri` - Connection URI (e.g., "s3://bucket-name" or "file:///local/path")
    /// * `table_name` - Name of the documents table
    pub async fn connect(uri: &str, table_name: &str) -> Result<Self> {
        let db = lancedb::connect(uri).execute().await?;

        Ok(Self {
            db: Arc::new(db),
            table_name: table_name.to_string(),
        })
    }

    /// Create the Arrow schema for document storage
    fn create_schema() -> Arc<Schema> {
        Arc::new(Schema::new(vec![
            Field::new("id", DataType::Utf8, false),
            Field::new("content", DataType::LargeUtf8, false),
            Field::new("title", DataType::Utf8, false),
            Field::new("author", DataType::Utf8, true),
            Field::new("version", DataType::Int32, false),
            Field::new(
                "tags",
                DataType::List(Arc::new(Field::new("item", DataType::Utf8, false))),
                false,
            ),
            Field::new("created_at", DataType::Int64, false),
            Field::new("updated_at", DataType::Int64, false),
            Field::new(
                "embedding",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, false)),
                    1024,
                ),
                true,
            ),
        ]))
    }

    /// Convert documents to Arrow RecordBatch
    fn documents_to_batch(documents: &[Document]) -> Result<RecordBatch> {
        use arrow_array::builder::{
            FixedSizeListBuilder, Float32Builder, Int32Builder, Int64Builder,
            LargeStringBuilder, ListBuilder, StringBuilder,
        };

        let schema = Self::create_schema();

        // Build arrays for each field
        let mut id_builder = StringBuilder::new();
        let mut content_builder = LargeStringBuilder::new();
        let mut title_builder = StringBuilder::new();
        let mut author_builder = StringBuilder::new();
        let mut version_builder = Int32Builder::new();
        let mut tags_builder = ListBuilder::new(StringBuilder::new());
        let mut created_at_builder = Int64Builder::new();
        let mut updated_at_builder = Int64Builder::new();
        let mut embedding_builder = FixedSizeListBuilder::new(Float32Builder::new(), 1024);

        for doc in documents {
            // Basic fields
            id_builder.append_value(&doc.id);
            content_builder.append_value(&doc.content);
            title_builder.append_value(&doc.metadata.title);

            // Nullable author
            if let Some(author) = &doc.metadata.author {
                author_builder.append_value(author);
            } else {
                author_builder.append_null();
            }

            version_builder.append_value(doc.metadata.version as i32);

            // Tags as List
            for tag in &doc.metadata.tags {
                tags_builder.values().append_value(tag);
            }
            tags_builder.append(true);

            created_at_builder.append_value(doc.metadata.created_at);
            updated_at_builder.append_value(doc.metadata.updated_at);

            // Embedding as FixedSizeList
            if let Some(emb) = &doc.embedding {
                for &val in emb {
                    embedding_builder.values().append_value(val);
                }
                embedding_builder.append(true);
            } else {
                // Append null values for the fixed size list
                for _ in 0..1024 {
                    embedding_builder.values().append_null();
                }
                embedding_builder.append(false);
            }
        }

        let id_array: ArrayRef = Arc::new(id_builder.finish());
        let content_array: ArrayRef = Arc::new(content_builder.finish());
        let title_array: ArrayRef = Arc::new(title_builder.finish());
        let author_array: ArrayRef = Arc::new(author_builder.finish());
        let version_array: ArrayRef = Arc::new(version_builder.finish());
        let tags_array: ArrayRef = Arc::new(tags_builder.finish());
        let created_at_array: ArrayRef = Arc::new(created_at_builder.finish());
        let updated_at_array: ArrayRef = Arc::new(updated_at_builder.finish());
        let embedding_array: ArrayRef = Arc::new(embedding_builder.finish());

        let batch = RecordBatch::try_new(
            schema,
            vec![
                id_array,
                content_array,
                title_array,
                author_array,
                version_array,
                tags_array,
                created_at_array,
                updated_at_array,
                embedding_array,
            ],
        )?;

        Ok(batch)
    }

    /// Convert RecordBatch to RecordBatchReader for LanceDB
    fn batch_to_reader(batch: RecordBatch) -> Result<Box<dyn RecordBatchReader + Send>> {
        struct SingleBatchReader {
            schema: Arc<Schema>,
            batch: Option<RecordBatch>,
        }

        impl Iterator for SingleBatchReader {
            type Item = std::result::Result<RecordBatch, arrow_schema::ArrowError>;

            fn next(&mut self) -> Option<Self::Item> {
                self.batch.take().map(Ok)
            }
        }

        impl RecordBatchReader for SingleBatchReader {
            fn schema(&self) -> Arc<Schema> {
                self.schema.clone()
            }
        }

        Ok(Box::new(SingleBatchReader {
            schema: batch.schema(),
            batch: Some(batch),
        }))
    }

    /// Convert RecordBatch rows to Documents
    fn batch_to_documents(batch: RecordBatch) -> Result<Vec<Document>> {
        let id_array = batch
            .column(0)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("Invalid id column"))?;

        let content_array = batch
            .column(1)
            .as_any()
            .downcast_ref::<LargeStringArray>()
            .ok_or_else(|| anyhow!("Invalid content column"))?;

        let title_array = batch
            .column(2)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("Invalid title column"))?;

        let author_array = batch
            .column(3)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| anyhow!("Invalid author column"))?;

        let version_array = batch
            .column(4)
            .as_any()
            .downcast_ref::<Int32Array>()
            .ok_or_else(|| anyhow!("Invalid version column"))?;

        let tags_array = batch
            .column(5)
            .as_any()
            .downcast_ref::<ListArray>()
            .ok_or_else(|| anyhow!("Invalid tags column"))?;

        let created_at_array = batch
            .column(6)
            .as_any()
            .downcast_ref::<Int64Array>()
            .ok_or_else(|| anyhow!("Invalid created_at column"))?;

        let updated_at_array = batch
            .column(7)
            .as_any()
            .downcast_ref::<Int64Array>()
            .ok_or_else(|| anyhow!("Invalid updated_at column"))?;

        let embedding_array = batch
            .column(8)
            .as_any()
            .downcast_ref::<FixedSizeListArray>()
            .ok_or_else(|| anyhow!("Invalid embedding column"))?;

        let mut documents = Vec::new();

        for i in 0..batch.num_rows() {
            let id = id_array.value(i).to_string();
            let content = content_array.value(i).to_string();
            let title = title_array.value(i).to_string();
            let author = if author_array.is_null(i) {
                None
            } else {
                Some(author_array.value(i).to_string())
            };
            let version = version_array.value(i) as u32;

            // Extract tags
            let tags_slice = tags_array.value(i);
            let tags_str_array = tags_slice
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| anyhow!("Invalid tags array"))?;
            let tags: Vec<String> = (0..tags_str_array.len())
                .map(|j| tags_str_array.value(j).to_string())
                .collect();

            let created_at = created_at_array.value(i);
            let updated_at = updated_at_array.value(i);

            // Extract embedding
            let embedding = if embedding_array.is_null(i) {
                None
            } else {
                let emb_slice = embedding_array.value(i);
                let emb_float_array = emb_slice
                    .as_any()
                    .downcast_ref::<arrow_array::Float32Array>()
                    .ok_or_else(|| anyhow!("Invalid embedding array"))?;
                let emb: Vec<f32> = (0..emb_float_array.len())
                    .map(|j| emb_float_array.value(j))
                    .collect();
                Some(emb)
            };

            documents.push(Document {
                id,
                content,
                metadata: DocumentMetadata {
                    title,
                    author,
                    version,
                    tags,
                    created_at,
                    updated_at,
                },
                embedding,
            });
        }

        Ok(documents)
    }

    /// Initialize the documents table if it doesn't exist
    pub async fn init_table(&self) -> Result<()> {
        // Table will be created on first insert if it doesn't exist
        // Schema: id (string), content (string), metadata_json (string), embedding (fixed_size_list[f32, 1024])
        Ok(())
    }

    /// Health check for storage connectivity
    pub async fn health_check(&self) -> Result<()> {
        // Try to list tables as a connectivity check
        let _tables = self.db.table_names().execute().await?;
        Ok(())
    }

    /// Count total documents in the corpus
    pub async fn count_documents(&self) -> Result<usize> {
        // Check if table exists first
        let table_names = self.db.table_names().execute().await?;
        if !table_names.contains(&self.table_name) {
            return Ok(0);
        }

        let table = self.db.open_table(&self.table_name).execute().await?;
        let count = table.count_rows(None).await?;
        Ok(count)
    }

    /// Upsert documents with embeddings into the corpus
    pub async fn upsert_documents(&self, documents: Vec<Document>) -> Result<()> {
        if documents.is_empty() {
            return Ok(());
        }

        tracing::info!("Upserting {} documents", documents.len());

        // Convert documents to Arrow RecordBatch
        let batch = Self::documents_to_batch(&documents)?;

        // Check if table exists
        let table_names = self.db.table_names().execute().await?;

        if table_names.contains(&self.table_name) {
            // Table exists - merge (upsert) the data
            let table = self.db.open_table(&self.table_name).execute().await?;
            let reader = Self::batch_to_reader(batch)?;
            let mut merge_builder = table.merge_insert(&["id"]);
            merge_builder.when_matched_update_all(None);
            merge_builder.when_not_matched_insert_all();
            merge_builder.execute(reader).await?;
        } else {
            // Table doesn't exist - create it
            let reader = Self::batch_to_reader(batch)?;
            self.db
                .create_table(&self.table_name, reader)
                .execute()
                .await?;
        }

        tracing::info!("Successfully upserted {} documents", documents.len());
        Ok(())
    }

    /// Perform vector similarity search
    ///
    /// # Arguments
    /// * `query_embedding` - The query vector (1024 dimensions for BGE-M3)
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    /// Vector of (Document, similarity_score) tuples sorted by descending score
    pub async fn vector_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<(Document, f32)>> {
        tracing::debug!(
            "Vector search with {} dimensions, limit {}",
            query_embedding.len(),
            limit
        );

        // Check if table exists
        let table_names = self.db.table_names().execute().await?;
        if !table_names.contains(&self.table_name) {
            tracing::warn!("Table {} does not exist yet", self.table_name);
            return Ok(vec![]);
        }

        // Open table
        let table = self.db.open_table(&self.table_name).execute().await?;

        // Perform vector search with cosine distance
        let query_result = table
            .query()
            .limit(limit)
            .nearest_to(query_embedding)?
            .distance_type(lancedb::DistanceType::Cosine)
            .execute()
            .await?;

        // Convert results to RecordBatch
        let batches = query_result.try_collect::<Vec<RecordBatch>>().await?;

        if batches.is_empty() {
            return Ok(vec![]);
        }

        // Process batches and extract documents with scores
        let mut results = Vec::new();

        for batch in batches {
            // Check if _distance column exists (LanceDB adds this for vector search)
            let distance_col_idx = batch
                .schema()
                .column_with_name("_distance")
                .map(|(idx, _)| idx);

            let documents = Self::batch_to_documents(batch.clone())?;

            // Extract distances if available
            if let Some(dist_idx) = distance_col_idx {
                let distance_array = batch
                    .column(dist_idx)
                    .as_any()
                    .downcast_ref::<arrow_array::Float32Array>()
                    .ok_or_else(|| anyhow!("Invalid distance column"))?;

                for (i, doc) in documents.into_iter().enumerate() {
                    // Convert cosine distance to similarity score (1 - distance)
                    let distance = distance_array.value(i);
                    let similarity = 1.0 - distance;
                    results.push((doc, similarity));
                }
            } else {
                // No distance column - assign default scores
                for doc in documents {
                    results.push((doc, 1.0));
                }
            }
        }

        // Sort by score descending
        results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        tracing::debug!("Found {} results", results.len());
        Ok(results)
    }

    /// Perform vector search with optional filters, returning SearchResult
    pub async fn vector_search_filtered(
        &self,
        query_embedding: &[f32],
        limit: usize,
        _filters: Option<&SearchFilters>,
    ) -> Result<Vec<SearchResult>> {
        // TODO: Implement filtered vector search
        let results = self.vector_search(query_embedding, limit).await?;

        Ok(results
            .into_iter()
            .map(|(doc, score)| SearchResult {
                document_id: doc.id,
                score,
                match_type: MatchType::from(score),
                snippet: doc.content.chars().take(200).collect(),
            })
            .collect())
    }

    /// Hybrid search combining vector similarity and BM25 keyword matching
    ///
    /// # Arguments
    /// * `query_embedding` - The query vector
    /// * `query_text` - The raw query text for keyword matching
    /// * `limit` - Maximum number of results
    pub async fn hybrid_search(
        &self,
        query_embedding: &[f32],
        query_text: &str,
        _limit: usize,
    ) -> Result<Vec<(Document, f32)>> {
        // TODO: Implement hybrid search with RRF fusion
        // This will be implemented in Phase 2.3
        tracing::debug!(
            "Hybrid search for '{}' with {} dimensions",
            query_text,
            query_embedding.len()
        );

        Ok(vec![])
    }

    /// Get a document by ID
    pub async fn get_document(&self, id: &str) -> Result<Option<Document>> {
        tracing::debug!("Getting document: {}", id);

        // Check if table exists
        let table_names = self.db.table_names().execute().await?;
        if !table_names.contains(&self.table_name) {
            return Ok(None);
        }

        // Open table
        let table = self.db.open_table(&self.table_name).execute().await?;

        // Query for the specific document by ID
        let filter_str = format!("id = '{}'", id);
        let query_result = table
            .query()
            .only_if(filter_str)
            .execute()
            .await?;

        // Convert results to RecordBatch
        let batches = query_result.try_collect::<Vec<RecordBatch>>().await?;

        if batches.is_empty() {
            return Ok(None);
        }

        // Extract the first document (should only be one with matching ID)
        for batch in batches {
            let documents = Self::batch_to_documents(batch)?;
            if let Some(doc) = documents.into_iter().next() {
                return Ok(Some(doc));
            }
        }

        Ok(None)
    }

    /// Get the current corpus version (for sync)
    pub async fn get_version(&self) -> Result<String> {
        // LanceDB tracks versions automatically
        // Check if table exists
        let table_names = self.db.table_names().execute().await?;
        if !table_names.contains(&self.table_name) {
            return Ok("0".to_string());
        }

        // Open table and get version
        let table = self.db.open_table(&self.table_name).execute().await?;
        let version = table.version().await?;

        Ok(version.to_string())
    }
}
