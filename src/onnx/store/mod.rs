pub mod github;
pub mod r2;

use crate::Result;
use clap::Subcommand;
use sha2::{Digest, Sha256};
use tokio::io::AsyncReadExt;

/// Trait defining the fetch operation for ONNX models.
pub trait Fetch {
    /// Asynchronously fetches an ONNX model.
    ///
    /// # Parameters
    /// - `model_name`: The name of the model to fetch.
    /// - `model_dir`: The directory where the model should be stored.
    /// - `update_check`: A boolean indicating whether to check for updates to the model.
    ///
    /// # Returns
    /// - A `Result` containing a `String`. On success, this `String` is the path to the fetched model.
    async fn fetch_model(
        &self,
        model_name: &'static str,
        model_dir: std::path::PathBuf,
        update_check: bool,
    ) -> Result<String>;
}

/// Enum representing the ONNX model storage options.
/// Currently, there are two options: R2 and Github.
#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum ONNXStore {
    /// Represents the CloudFlare R2 storage option.
    R2 {
        /// The name of the bucket.
        #[clap(short, long)]
        bucket_name: String,

        /// The URI of the Cloudflare KV.
        #[clap(short, long)]
        cloudflare_kv_uri: String,

        /// The client ID of the Cloudflare KV.
        #[clap(short, long)]
        cloudflare_kv_client_id: String,

        /// The secret of the Cloudflare KV.
        #[clap(short, long)]
        cloudflare_kv_secret: String,
    },
    /// Represents the Github storage option.
    Github,
}

impl Default for ONNXStore {
    fn default() -> Self {
        ONNXStore::Github
    }
}

pub enum FetchStore {
    R2(r2::R2Store),
    Github(github::GithubStore),
}

impl FetchStore {
    pub async fn new(onnx_store: ONNXStore) -> Self {
        match onnx_store {
            ONNXStore::R2 {
                bucket_name,
                cloudflare_kv_uri,
                cloudflare_kv_client_id,
                cloudflare_kv_secret,
            } => {
                let r2 = r2::R2Store::new(
                    bucket_name,
                    cloudflare_kv_uri,
                    cloudflare_kv_client_id,
                    cloudflare_kv_secret,
                )
                .await;
                FetchStore::R2(r2)
            }
            ONNXStore::Github => FetchStore::Github(github::GithubStore),
        }
    }
}

impl Fetch for FetchStore {
    async fn fetch_model(
        &self,
        model_name: &'static str,
        model_dir: std::path::PathBuf,
        update_check: bool,
    ) -> Result<String> {
        match self {
            FetchStore::R2(r2) => r2.fetch_model(model_name, model_dir, update_check).await,
            FetchStore::Github(github) => {
                github
                    .fetch_model(model_name, model_dir, update_check)
                    .await
            }
        }
    }
}

async fn file_sha256(filename: &str) -> Result<String> {
    let mut file = tokio::fs::File::open(filename).await?;
    let mut sha256 = Sha256::new();
    let mut buffer = [0; 1024];
    while let Ok(n) = file.read(&mut buffer).await {
        if n == 0 {
            break;
        }
        sha256.update(&buffer[..n]);
    }
    Ok(format!("{:x}", sha256.finalize()))
}
