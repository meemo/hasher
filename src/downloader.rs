use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use futures::{stream::BoxStream, StreamExt};
use log::{debug, info, trace, warn};
use reqwest::Client;

use crate::compression::{self, CompressionAlgorithm};

#[derive(Clone)]
pub struct DownloadConfig {
    pub retry_count: u32,
    pub retry_delay: Duration,
    pub compress: bool,
    pub compression_level: u32,
    pub no_clobber: bool,
}

impl Default for DownloadConfig {
    fn default() -> Self {
        Self {
            retry_count: 3,
            retry_delay: Duration::from_secs(5),
            compress: false,
            compression_level: 6,
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
        if !self.config.compress {
            return Ok(buffer);
        }

        let compressor = compression::get_compressor(
            compression::CompressionType::Gzip,
            self.config.compression_level,
        );

        tokio::task::spawn_blocking(move || {
            let mut writer = Vec::new();
            compressor.compress_file(&mut std::io::Cursor::new(&buffer), &mut writer)?;
            Ok(writer)
        })
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?
    }

    async fn attempt_download(
        &self,
        url: &str,
        dest_path: &Path,
    ) -> Result<(u64, PathBuf), Box<dyn std::error::Error>> {
        debug!("Attempting download of {}", url);
        trace!("Destination path: {}", dest_path.display());

        let response = self.client.get(url).send().await?;
        response.error_for_status_ref()?;

        let total_size = response.content_length().unwrap_or(0);
        debug!("Content length: {} bytes", total_size);
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
            trace!("Downloaded chunk of {} bytes", chunk.len());
        }

        debug!("Download complete, processing buffer");
        let processed_buffer = self.process_download_buffer(buffer).await?;
        debug!(
            "Writing {} bytes to {}",
            processed_buffer.len(),
            dest_path.display()
        );
        tokio::fs::write(dest_path, processed_buffer).await?;

        Ok((downloaded, dest_path.to_path_buf()))
    }

    pub async fn download_file(&self, url: String, dest_path: PathBuf) -> DownloadResult {
        debug!("Starting download for {}", url);
        let mut result = DownloadResult {
            url: url.clone(),
            path: dest_path,
            size: 0,
            success: false,
            error: None,
        };

        // Check if file exists when no-clobber is enabled
        if self.config.no_clobber && result.path.exists() {
            debug!("File exists and --no-clobber is enabled, skipping {}", url);
            result.success = true;
            result.error = Some("File exists, skipping download".to_string());
            // Get the file size for skipped files
            if let Ok(metadata) = std::fs::metadata(&result.path) {
                result.size = metadata.len();
            }
            return result;
        }

        // Create parent directories before attempting download
        if let Some(parent) = result.path.parent() {
            debug!("Creating parent directories: {}", parent.display());
            if let Err(e) = tokio::fs::create_dir_all(parent).await {
                result.error = Some(format!("Failed to create directories: {}", e));
                return result;
            }
        }

        for attempt in 0..=self.config.retry_count {
            if attempt > 0 {
                debug!("Retry attempt {} for {}", attempt, &url);
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
    ) -> BoxStream<'_, DownloadResult>
    where
        P: AsRef<Path>,
        F: Fn(&str) -> PathBuf + Send + Sync + 'static,
    {
        let dest_dir = dest_dir.as_ref().to_path_buf();

        if let Err(e) = tokio::fs::create_dir_all(&dest_dir).await {
            return futures::stream::once(async move {
                DownloadResult {
                    url: "".to_string(),
                    path: dest_dir,
                    size: 0,
                    success: false,
                    error: Some(format!("Failed to create directory: {}", e)),
                }
            })
            .boxed();
        }

        futures::stream::iter(urls)
            .map(move |url| {
                let dest_path = dest_dir.join(filename_fn(&url));
                async move { self.download_file(url, dest_path).await }
            })
            .buffer_unordered(1)
            .boxed()
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
