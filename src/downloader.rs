use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use futures::StreamExt;
use log::{info, warn};
use reqwest::Client;

use crate::compression::{self, CompressionAlgorithm};

#[derive(Clone)]
pub struct DownloadConfig {
    pub retry_count: u32,
    pub retry_delay: Duration,
    pub concurrent_downloads: usize,
    pub _chunk_size: usize,
    pub compress: bool,
    pub compression_level: u32,
    pub _hash_compressed: bool,
    pub no_clobber: bool,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            retry_count: 3,
            retry_delay: Duration::from_secs(5),
            concurrent_downloads: 4,
            _chunk_size: 1024 * 1024, // 1MB chunks
            compress: false,
            compression_level: 6,
            _hash_compressed: false,
            no_clobber: false,
        }
    }
}

#[derive(Debug)]
pub struct DownloadResult {
    pub url: String,
    pub path: PathBuf,
    pub size: u64,
    pub success: bool,
    pub error: Option<String>,
}

pub struct Downloader {
    client: Client,
    config: DownloadConfig,
}

impl Downloader {
    pub fn new(config: DownloadConfig) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .expect("Failed to create HTTP client");

        Self { client, config }
    }

    async fn process_download_buffer(&self, buffer: Vec<u8>) -> io::Result<Vec<u8>> {
        if self.config.compress {
            let compressor = compression::get_compressor(
                compression::CompressionType::Gzip,
                self.config.compression_level,
            );

            tokio::task::spawn_blocking(move || {
                use std::io::Cursor;
                let mut reader = Cursor::new(&buffer);
                let mut writer = Vec::new();
                compressor
                    .compress_file(&mut reader, &mut writer)
                    .map(|_| writer)
            })
            .await
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
        } else {
            Ok(buffer)
        }
    }

    async fn attempt_download(
        &self,
        url: &str,
        dest_path: &Path,
    ) -> Result<(u64, PathBuf), Box<dyn std::error::Error>> {
        let response = self.client.get(url).send().await?;
        response.error_for_status_ref()?;

        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded = 0u64;
        let mut buffer = Vec::new();
        let mut stream = response.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            buffer.extend_from_slice(&chunk);
            downloaded += chunk.len() as u64;

            if downloaded % (5 * 1024 * 1024) == 0 {
                info!("Downloaded {}/{} bytes for {}", downloaded, total_size, url);
            }
        }

        let processed_buffer = self.process_download_buffer(buffer).await?;
        tokio::fs::write(dest_path, processed_buffer).await?;

        Ok((downloaded, dest_path.to_path_buf()))
    }

    pub async fn download_file(&self, url: String, dest_path: PathBuf) -> DownloadResult {
        let mut result = DownloadResult {
            url,
            path: dest_path,
            size: 0,
            success: false,
            error: None,
        };

        // Check if file exists when no-clobber is enabled
        if self.config.no_clobber && result.path.exists() {
            result.success = true;
            result.error = Some("File exists, skipping download".to_string());
            return result;
        }

        for attempt in 0..=self.config.retry_count {
            if attempt > 0 {
                tokio::time::sleep(self.config.retry_delay).await;
            }

            match self.attempt_download(&result.url, &result.path).await {
                Ok((size, final_path)) => {
                    result.size = size;
                    result.path = final_path;
                    result.success = true;
                    break;
                }
                Err(e) => {
                    result.error = Some(e.to_string());
                    if attempt == self.config.retry_count {
                        warn!(
                            "Failed to download {} after {} attempts: {}",
                            result.url,
                            attempt + 1,
                            e
                        );
                    }
                }
            }
        }

        result
    }

    pub async fn download_from_list<P, F>(
        &self,
        urls: Vec<String>,
        dest_dir: P,
        filename_fn: F,
    ) -> Vec<DownloadResult>
    where
        P: AsRef<Path>,
        F: Fn(&str) -> PathBuf + Send + Sync + 'static,
    {
        let dest_dir = dest_dir.as_ref().to_path_buf();
        tokio::fs::create_dir_all(&dest_dir).await.ok();

        futures::stream::iter(urls)
            .map(|url| {
                let dest_path = dest_dir.join(filename_fn(&url));
                self.download_file(url.clone(), dest_path)
            })
            .buffer_unordered(self.config.concurrent_downloads)
            .collect()
            .await
    }
}

pub fn sanitize_filename(url: &str) -> PathBuf {
    let filename = url.split('/').last().unwrap_or("download");

    let clean_name: String = filename
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    if clean_name.is_empty() {
        PathBuf::from("download")
    } else {
        PathBuf::from(clean_name)
    }
}
