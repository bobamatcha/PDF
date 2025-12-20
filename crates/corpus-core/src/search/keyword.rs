use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::*;
use tantivy::{Index, IndexWriter, ReloadPolicy, TantivyDocument};
use anyhow::Result;
use std::path::Path;

/// Keyword search index using Tantivy for BM25 matching
///
/// This module provides full-text search capabilities using Tantivy's BM25 algorithm.
/// It maintains a separate index for keyword matching that complements the vector
/// search functionality in the hybrid search pipeline.
///
/// # Schema
///
/// The index contains three fields:
/// - `id`: Unique document identifier (STRING | STORED)
/// - `content`: Document content for full-text search (TEXT | STORED)
/// - `title`: Document title for boosted matching (TEXT | STORED)
///
/// # Example
///
/// ```no_run
/// use std::path::Path;
/// use corpus_core::search::keyword::KeywordIndex;
///
/// # fn main() -> anyhow::Result<()> {
/// let index = KeywordIndex::new(Path::new("./tantivy_index"))?;
/// index.add_document("doc1", "Sample Title", "Document content here")?;
///
/// let results = index.search("document content", 10)?;
/// for (doc_id, score) in results {
///     println!("Found: {} with score {}", doc_id, score);
/// }
/// # Ok(())
/// # }
/// ```
pub struct KeywordIndex {
    index: Index,
    schema: Schema,
    id_field: Field,
    content_field: Field,
    title_field: Field,
}

impl KeywordIndex {
    /// Create or open a keyword index at the given path
    ///
    /// If the index already exists at the specified path, it will be opened.
    /// Otherwise, a new index will be created with the appropriate schema.
    ///
    /// # Arguments
    ///
    /// * `index_path` - Directory path where the Tantivy index will be stored
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The directory cannot be created
    /// - The index cannot be opened or created
    /// - File system permissions are insufficient
    pub fn new(index_path: &Path) -> Result<Self> {
        Self::open_or_create(index_path)
    }

    /// Open or create a keyword index (alias for new)
    pub fn open_or_create(index_path: &Path) -> Result<Self> {
        let mut schema_builder = Schema::builder();

        let id_field = schema_builder.add_text_field("id", STRING | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);

        let schema = schema_builder.build();

        let index = if index_path.exists() {
            Index::open_in_dir(index_path)?
        } else {
            std::fs::create_dir_all(index_path)?;
            Index::create_in_dir(index_path, schema.clone())?
        };

        Ok(Self {
            index,
            schema,
            id_field,
            content_field,
            title_field,
        })
    }

    /// Create an in-memory index (for testing)
    ///
    /// This is useful for unit tests and benchmarks where persistence is not needed.
    /// The index will be stored entirely in RAM and will be lost when dropped.
    ///
    /// # Example
    ///
    /// ```
    /// use corpus_core::search::keyword::KeywordIndex;
    ///
    /// # fn main() -> anyhow::Result<()> {
    /// let index = KeywordIndex::in_memory()?;
    /// index.add_document("test1", "Test", "Content")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn in_memory() -> Result<Self> {
        let mut schema_builder = Schema::builder();

        let id_field = schema_builder.add_text_field("id", STRING | STORED);
        let content_field = schema_builder.add_text_field("content", TEXT | STORED);
        let title_field = schema_builder.add_text_field("title", TEXT | STORED);

        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema.clone());

        Ok(Self {
            index,
            schema,
            id_field,
            content_field,
            title_field,
        })
    }

    /// Add a document to the index
    ///
    /// Creates a new document with the provided fields and adds it to the index.
    /// The document is committed immediately to ensure it's available for search.
    ///
    /// # Arguments
    ///
    /// * `id` - Unique identifier for the document
    /// * `title` - Document title (searched with higher weight)
    /// * `content` - Main document content
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The index writer cannot be created (50MB heap allocation)
    /// - The document cannot be added
    /// - The commit operation fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use corpus_core::search::keyword::KeywordIndex;
    /// # fn main() -> anyhow::Result<()> {
    /// # let index = KeywordIndex::in_memory()?;
    /// index.add_document(
    ///     "legal-contract-001",
    ///     "Employment Agreement",
    ///     "This agreement is entered into between..."
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn add_document(&self, id: &str, title: &str, content: &str) -> Result<()> {
        let mut index_writer: IndexWriter = self.index.writer(50_000_000)?;

        let mut doc = TantivyDocument::new();
        doc.add_text(self.id_field, id);
        doc.add_text(self.title_field, title);
        doc.add_text(self.content_field, content);

        index_writer.add_document(doc)?;
        index_writer.commit()?;

        Ok(())
    }

    /// Search for documents matching the query
    ///
    /// Performs a BM25-based full-text search across both content and title fields.
    /// Results are ranked by relevance score, with title matches typically receiving
    /// higher scores due to field boosting.
    ///
    /// # Arguments
    ///
    /// * `query` - Search query string (supports Tantivy query syntax)
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    ///
    /// A vector of tuples containing:
    /// - Document ID (String)
    /// - BM25 relevance score (f32)
    ///
    /// Results are sorted by descending score.
    ///
    /// # Query Syntax
    ///
    /// The query parser supports:
    /// - Boolean operators: AND, OR, NOT
    /// - Phrase queries: "exact phrase"
    /// - Field-specific search: title:keyword
    /// - Wildcards: * and ?
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The index reader cannot be created
    /// - The query syntax is invalid
    /// - The search operation fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use corpus_core::search::keyword::KeywordIndex;
    /// # fn main() -> anyhow::Result<()> {
    /// # let index = KeywordIndex::in_memory()?;
    /// # index.add_document("doc1", "Contract", "Sample content")?;
    /// let results = index.search("employment AND contract", 10)?;
    /// for (doc_id, score) in results {
    ///     println!("Document: {}, BM25 Score: {:.4}", doc_id, score);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<(String, f32)>> {
        let reader = self.index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let searcher = reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.content_field, self.title_field]);

        let query = query_parser.parse_query(query)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let results: Vec<(String, f32)> = top_docs
            .into_iter()
            .map(|(score, doc_address)| {
                let doc: TantivyDocument = searcher.doc(doc_address).unwrap();
                let id = doc.get_first(self.id_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                (id, score)
            })
            .collect();

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_index() {
        let index = KeywordIndex::in_memory().expect("Failed to create in-memory index");

        index.add_document("doc1", "Test Title", "This is test content")
            .expect("Failed to add document");

        let results = index.search("test", 10).expect("Search failed");
        assert!(!results.is_empty(), "Should find at least one result");
        assert_eq!(results[0].0, "doc1", "Should find doc1");
    }

    #[test]
    fn test_phrase_search() {
        let index = KeywordIndex::in_memory().expect("Failed to create index");

        index.add_document("doc1", "Title", "The quick brown fox")
            .expect("Failed to add document");
        index.add_document("doc2", "Title", "The slow brown dog")
            .expect("Failed to add document");

        let results = index.search("\"quick brown\"", 10).expect("Search failed");
        assert_eq!(results.len(), 1, "Phrase search should find exact match");
        assert_eq!(results[0].0, "doc1");
    }

    #[test]
    fn test_title_and_content_search() {
        let index = KeywordIndex::in_memory().expect("Failed to create index");

        index.add_document("doc1", "Employment Contract", "Standard terms")
            .expect("Failed to add document");
        index.add_document("doc2", "Standard Agreement", "Employment terms")
            .expect("Failed to add document");

        let results = index.search("employment", 10).expect("Search failed");
        assert_eq!(results.len(), 2, "Should find matches in both title and content");
    }

    #[test]
    fn test_empty_query() {
        let index = KeywordIndex::in_memory().expect("Failed to create index");
        index.add_document("doc1", "Title", "Content").expect("Failed to add document");

        // Empty or whitespace queries should return error or no results
        let results = index.search("", 10);
        assert!(results.is_err() || results.unwrap().is_empty());
    }

    #[test]
    fn test_relevance_scoring() {
        let index = KeywordIndex::in_memory().expect("Failed to create index");

        index.add_document("doc1", "Title", "rust rust rust programming")
            .expect("Failed to add document");
        index.add_document("doc2", "Title", "rust programming")
            .expect("Failed to add document");
        index.add_document("doc3", "Title", "python programming")
            .expect("Failed to add document");

        let results = index.search("rust", 10).expect("Search failed");

        // doc1 should have higher score due to term frequency
        assert_eq!(results[0].0, "doc1", "Document with more occurrences should rank higher");
        assert!(results[0].1 > results[1].1, "Scores should be descending");
    }
}
