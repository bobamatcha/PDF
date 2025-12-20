//! Configuration management for the corpus system
//!
//! Handles storage provider configuration and object store setup.

use anyhow::{anyhow, Result};
use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::ObjectStore;
use std::sync::Arc;

/// Storage provider options
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageProvider {
    /// Cloudflare R2 (recommended for zero egress fees)
    CloudflareR2,
    /// AWS S3
    AwsS3,
    /// Self-hosted MinIO
    MinIO,
    /// Local filesystem (for development)
    Local,
}

/// Storage configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// The storage provider to use
    pub provider: StorageProvider,
    /// Bucket name (or local path for Local provider)
    pub bucket: String,
    /// AWS region (use "auto" for R2)
    pub region: String,
    /// Custom endpoint URL (required for R2 and MinIO)
    pub endpoint: Option<String>,
    /// AWS access key ID
    pub access_key_id: Option<String>,
    /// AWS secret access key
    pub secret_access_key: Option<String>,
}

impl StorageConfig {
    /// Create a new configuration for Cloudflare R2
    pub fn r2(bucket: &str, account_id: &str) -> Self {
        Self {
            provider: StorageProvider::CloudflareR2,
            bucket: bucket.to_string(),
            region: "auto".to_string(),
            endpoint: Some(format!("https://{}.r2.cloudflarestorage.com", account_id)),
            access_key_id: None,
            secret_access_key: None,
        }
    }

    /// Create a new configuration for AWS S3
    pub fn s3(bucket: &str, region: &str) -> Self {
        Self {
            provider: StorageProvider::AwsS3,
            bucket: bucket.to_string(),
            region: region.to_string(),
            endpoint: None,
            access_key_id: None,
            secret_access_key: None,
        }
    }

    /// Create a new configuration for local filesystem
    pub fn local(path: &str) -> Self {
        Self {
            provider: StorageProvider::Local,
            bucket: path.to_string(),
            region: String::new(),
            endpoint: None,
            access_key_id: None,
            secret_access_key: None,
        }
    }

    /// Set AWS credentials
    pub fn with_credentials(mut self, access_key_id: &str, secret_access_key: &str) -> Self {
        self.access_key_id = Some(access_key_id.to_string());
        self.secret_access_key = Some(secret_access_key.to_string());
        self
    }

    /// Load configuration from environment variables
    ///
    /// Expected variables:
    /// - CORPUS_STORAGE_PROVIDER: "cloudflare_r2", "aws_s3", "minio", or "local"
    /// - CORPUS_BUCKET: Bucket name or local path
    /// - CORPUS_REGION: AWS region (default: "auto" for R2)
    /// - R2_ENDPOINT or MINIO_ENDPOINT: Custom endpoint URL
    /// - AWS_ACCESS_KEY_ID: Access key
    /// - AWS_SECRET_ACCESS_KEY: Secret key
    pub fn from_env() -> Result<Self> {
        let provider_str = std::env::var("CORPUS_STORAGE_PROVIDER")
            .unwrap_or_else(|_| "local".to_string());

        let provider = match provider_str.to_lowercase().as_str() {
            "cloudflare_r2" | "r2" => StorageProvider::CloudflareR2,
            "aws_s3" | "s3" => StorageProvider::AwsS3,
            "minio" => StorageProvider::MinIO,
            "local" => StorageProvider::Local,
            _ => return Err(anyhow!("Unknown storage provider: {}", provider_str)),
        };

        let bucket = std::env::var("CORPUS_BUCKET")
            .unwrap_or_else(|_| "./corpus-data".to_string());

        let region = std::env::var("CORPUS_REGION")
            .unwrap_or_else(|_| "auto".to_string());

        let endpoint = std::env::var("R2_ENDPOINT")
            .or_else(|_| std::env::var("MINIO_ENDPOINT"))
            .ok();

        let access_key_id = std::env::var("AWS_ACCESS_KEY_ID").ok();
        let secret_access_key = std::env::var("AWS_SECRET_ACCESS_KEY").ok();

        Ok(Self {
            provider,
            bucket,
            region,
            endpoint,
            access_key_id,
            secret_access_key,
        })
    }

    /// Build an ObjectStore instance from this configuration
    pub fn build_object_store(&self) -> Result<Arc<dyn ObjectStore>> {
        match &self.provider {
            StorageProvider::CloudflareR2 | StorageProvider::MinIO => {
                let endpoint = self.endpoint.as_ref()
                    .ok_or_else(|| anyhow!("Endpoint required for {:?}", self.provider))?;

                let mut builder = AmazonS3Builder::new()
                    .with_bucket_name(&self.bucket)
                    .with_region(&self.region)
                    .with_endpoint(endpoint)
                    .with_virtual_hosted_style_request(false);

                if let (Some(key), Some(secret)) = (&self.access_key_id, &self.secret_access_key) {
                    builder = builder
                        .with_access_key_id(key)
                        .with_secret_access_key(secret);
                }

                Ok(Arc::new(builder.build()?))
            }

            StorageProvider::AwsS3 => {
                let mut builder = AmazonS3Builder::new()
                    .with_bucket_name(&self.bucket)
                    .with_region(&self.region);

                if let (Some(key), Some(secret)) = (&self.access_key_id, &self.secret_access_key) {
                    builder = builder
                        .with_access_key_id(key)
                        .with_secret_access_key(secret);
                }

                Ok(Arc::new(builder.build()?))
            }

            StorageProvider::Local => {
                // Ensure directory exists
                std::fs::create_dir_all(&self.bucket)?;
                Ok(Arc::new(LocalFileSystem::new_with_prefix(&self.bucket)?))
            }
        }
    }

    /// Generate the LanceDB connection URI for this configuration
    pub fn lance_uri(&self) -> String {
        match self.provider {
            StorageProvider::Local => format!("{}", self.bucket),
            _ => format!("s3://{}", self.bucket),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_config() {
        let config = StorageConfig::local("/tmp/test-corpus");
        assert_eq!(config.provider, StorageProvider::Local);
        assert_eq!(config.bucket, "/tmp/test-corpus");
        assert_eq!(config.lance_uri(), "/tmp/test-corpus");
    }

    #[test]
    fn test_r2_config() {
        let config = StorageConfig::r2("my-bucket", "abc123");
        assert_eq!(config.provider, StorageProvider::CloudflareR2);
        assert_eq!(config.region, "auto");
        assert!(config.endpoint.unwrap().contains("abc123"));
    }

    #[test]
    fn test_s3_config() {
        let config = StorageConfig::s3("my-bucket", "us-east-1");
        assert_eq!(config.provider, StorageProvider::AwsS3);
        assert_eq!(config.region, "us-east-1");
        assert_eq!(config.lance_uri(), "s3://my-bucket");
    }
}
