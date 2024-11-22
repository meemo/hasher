use std::path::{Path, PathBuf};

use log::info;
use reqwest::Url;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::compression::{self, CompressionAlgorithm};
use crate::configuration::{Config, HasherDownloadArgs};
use crate::downloader::{sanitize_filename, DownloadConfig, Downloader};
use crate::utils::Error;

fn construct_download_path(url: &str, base_dir: &Path) -> Result<PathBuf, Error> {
    let parsed = Url::parse(url).map_err(|e| Error::Download(format!("Invalid URL: {}", e)))?;
    let host = parsed
        .host_str()
        .ok_or_else(|| Error::Download("No host in URL".to_string()))?;

    let path = parsed.path();
    if path.is_empty() || path == "/" {
        return Err(Error::Download("No file path in URL".to_string()));
    }

    let mut segments: Vec<_> = path.split('/').filter(|s| !s.is_empty()).collect();

    if segments.is_empty() {
        return Err(Error::Download("No valid path segments in URL".to_string()));
    }

    let mut full_path = base_dir.to_path_buf();
    full_path.push(host);

    let filename = segments.pop().unwrap();

    for segment in segments {
        full_path.push(segment);
    }

    full_path.push(sanitize_filename(filename));
    Ok(full_path)
}

async fn read_url_list(path: &Path) -> Result<Vec<String>, Error> {
    let file = File::open(path).await?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut urls = Vec::new();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if !line.is_empty() {
            urls.push(line.to_string());
        }
    }

    Ok(urls)
}

async fn process_download_result(
    result: crate::downloader::DownloadResult,
    args: &HasherDownloadArgs,
    config: &Config,
) -> Result<(), Error> {
    if result.success {
        info!(
            "Successfully downloaded {} to {}",
            result.url,
            result.path.display()
        );

        if !args.hash_options.dry_run {
            crate::output::process_single_file(&result.path, config, &args.hash_options, &mut None)
                .await?;
        }
    } else if !args.hash_options.continue_on_error {
        return Err(Error::Download(format!(
            "Failed to download {}: {}",
            result.url,
            result.error.unwrap_or_else(|| "Unknown error".to_string())
        )));
    }

    Ok(())
}

pub async fn execute(args: HasherDownloadArgs, config: &Config) -> Result<(), Error> {
    let download_config = DownloadConfig {
        compress: args.hash_options.compress,
        compression_level: args.hash_options.compression_level,
        _hash_compressed: args.hash_options.hash_compressed,
        no_clobber: args.no_clobber,
        ..DownloadConfig::default()
    };

    let downloader = Downloader::new(download_config.clone());

    let urls = if args.source.exists() {
        read_url_list(&args.source).await?
    } else {
        vec![args.source.to_string_lossy().to_string()]
    };

    let download_config = download_config;
    let results = downloader
        .download_from_list(urls, &args.destination, move |url| {
            let mut path = match construct_download_path(url, Path::new("")) {
                Ok(p) => p,
                Err(_) => return sanitize_filename(url),
            };

            if download_config.compress {
                let compressor = compression::get_compressor(
                    compression::CompressionType::Gzip,
                    download_config.compression_level,
                );
                path = path.with_extension(format!(
                    "{}{}",
                    path.extension().unwrap_or_default().to_string_lossy(),
                    compressor.extension()
                ));
            }
            path
        })
        .await;

    for result in results {
        process_download_result(result, &args, config).await?;
    }

    Ok(())
}
