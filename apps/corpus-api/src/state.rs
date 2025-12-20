//! Application state for the Corpus Server
//!
//! Holds shared state including storage, embedding model, and search indices.

use anyhow::Result;
use corpus_core::config::StorageConfig;
use corpus_core::embeddings::EmbeddingModel;
use corpus_core::search::{KeywordIndex, VectorSearch};
use corpus_core::storage::CorpusStorage;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

/// Shared application state
pub struct AppState {
    /// LanceDB storage backend
    pub storage: Arc<CorpusStorage>,
    /// Embedding model for vector search
    pub embeddings: Arc<EmbeddingModel>,
    /// Keyword search index
    pub keyword_index: Arc<RwLock<KeywordIndex>>,
    /// Vector search interface
    pub vector_search: Arc<VectorSearch>,
    /// Current corpus version
    pub corpus_version: Arc<RwLock<String>>,
}

impl AppState {
    /// Initialize application state from environment configuration
    pub async fn new() -> Result<Self> {
        // Load storage configuration
        let storage_config = StorageConfig::from_env()?;
        info!("Connecting to storage: {:?}", storage_config.provider);

        // Initialize LanceDB storage
        let storage = Arc::new(
            CorpusStorage::connect(&storage_config.lance_uri(), "documents").await?
        );

        // Load embedding model
        let model_path = std::env::var("EMBEDDING_MODEL_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./models/bge-m3"));

        info!("Loading embedding model from {:?}", model_path);
        let embeddings = Arc::new(EmbeddingModel::load(&model_path).await?);
        info!("Embedding model loaded, dimension: {}", embeddings.dimension());

        // Initialize keyword index
        let index_path = std::env::var("KEYWORD_INDEX_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data/keyword_index"));

        let keyword_index = Arc::new(RwLock::new(
            KeywordIndex::open_or_create(&index_path)?
        ));

        // Initialize vector search
        let vector_search = Arc::new(VectorSearch::new(Arc::clone(&storage)));

        // Get current corpus version
        let corpus_version = Arc::new(RwLock::new(
            storage.get_version().await.unwrap_or_else(|_| "0.0.0".to_string())
        ));

        Ok(Self {
            storage,
            embeddings,
            keyword_index,
            vector_search,
            corpus_version,
        })
    }
}
