//! Application state for DocSign API

use anyhow::Result;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::path::PathBuf;

pub struct AppState {
    pub db: SqlitePool,
}

impl AppState {
    pub async fn new() -> Result<Self> {
        // Get database path from env or use default
        let db_path = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            let data_dir = dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("docsign-api");
            std::fs::create_dir_all(&data_dir).ok();
            format!("sqlite:{}/docsign.db?mode=rwc", data_dir.display())
        });

        tracing::info!("Connecting to database: {}", db_path);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&db_path)
            .await?;

        // Run migrations
        Self::run_migrations(&pool).await?;

        Ok(Self { db: pool })
    }

    async fn run_migrations(pool: &SqlitePool) -> Result<()> {
        tracing::info!("Running database migrations...");

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                document_name TEXT NOT NULL,
                document_hash TEXT NOT NULL,
                pdf_data BLOB NOT NULL,
                recipients_json TEXT NOT NULL,
                fields_json TEXT NOT NULL,
                signatures_json TEXT NOT NULL DEFAULT '{}',
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now')),
                expires_at TEXT
            )
            "#,
        )
        .execute(pool)
        .await?;

        // Index for fast lookups
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status)
            "#,
        )
        .execute(pool)
        .await?;

        tracing::info!("Migrations complete");
        Ok(())
    }
}

/// Get platform-specific data directory
mod dirs {
    use std::path::PathBuf;

    pub fn data_dir() -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            std::env::var("HOME")
                .ok()
                .map(|h| PathBuf::from(h).join("Library/Application Support"))
        }
        #[cfg(target_os = "linux")]
        {
            std::env::var("XDG_DATA_HOME")
                .ok()
                .map(PathBuf::from)
                .or_else(|| {
                    std::env::var("HOME")
                        .ok()
                        .map(|h| PathBuf::from(h).join(".local/share"))
                })
        }
        #[cfg(target_os = "windows")]
        {
            std::env::var("APPDATA").ok().map(PathBuf::from)
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
        {
            None
        }
    }
}
