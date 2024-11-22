use std::path::{Path, PathBuf};

use log::info;
use reqwest::Url;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::compression::{self, CompressionAlgorithm, CompressionType};
use crate::configuration::{Config, HasherDownloadArgs};
use crate::downloader::{sanitize_filename, DownloadConfig, DownloadResult, Downloader};
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

fn build_download_config(args: &HasherDownloadArgs) -> DownloadConfig {
    DownloadConfig {
        retry_count: args.hash_options.retry_count,
        retry_delay: std::time::Duration::from_secs(args.hash_options.retry_delay as u64),
        compress: args.hash_options.compress,
        compression_level: args.hash_options.compression_level,
        _hash_compressed: args.hash_options.hash_compressed,
        no_clobber: args.no_clobber,
        _chunk_size: 1024 * 1024,
    }
}

fn build_failure_json(result: &DownloadResult) -> String {
    serde_json::json!({
        "url": result.url,
        "destination": result.path,
        "error": result.error.clone().unwrap_or_else(|| "Unknown error".to_string()),
        "type": "download_failure"
    }).to_string()
}

async fn process_download_result(
    result: DownloadResult,
    args: &HasherDownloadArgs,
    config: &Config,
) -> Result<bool, Error> {
    if result.success {
        info!(
            "Successfully downloaded {} to {}",
            result.url,
            result.path.display()
        );
        if !args.hash_options.dry_run {
            match crate::output::process_single_file(
                &result.path,
                config,
                &args.hash_options,
                &mut None,
            )
            .await
            {
                Ok(()) => Ok(false),
                Err(e) if args.hash_options.skip_failures => {
                    println!("{}", serde_json::json!({
                        "url": result.url,
                        "destination": result.path,
                        "error": e.to_string(),
                        "type": "hash_failure"
                    }));
                    Ok(true)
                }
                Err(e) => Err(e),
            }
        } else {
            Ok(false)
        }
    } else if args.hash_options.skip_failures {
        println!("{}", build_failure_json(&result));
        Ok(true)
    } else {
        Err(Error::Download(format!(
            "Failed to download {}: {}",
            result.url,
            result.error.unwrap_or_else(|| "Unknown error".to_string())
        )))
    }
}

pub async fn execute(args: HasherDownloadArgs, config: &Config) -> Result<(), Error> {
    let urls = if args.source.exists() {
        read_url_list(&args.source).await?
    } else {
        vec![args.source.to_string_lossy().to_string()]
    };

    let should_compress = args.hash_options.compress;
    let compression_level = args.hash_options.compression_level;
    let downloader = Downloader::new(build_download_config(&args));

    let results = downloader
        .download_from_list(urls, &args.destination, move |url| {
            if let Ok(path) = construct_download_path(url, Path::new("")) {
                if should_compress {
                    let compressor = compression::get_compressor(
                        CompressionType::Gzip,
                        compression_level,
                    );
                    path.with_extension(format!(
                        "{}{}",
                        path.extension().unwrap_or_default().to_string_lossy(),
                        compressor.extension()
                    ))
                } else {
                    path
                }
            } else {
                Path::new("").join(sanitize_filename(url))
            }
        })
        .await;

    let mut had_failures = false;
    for result in results {
        match process_download_result(result, &args, config).await {
            Ok(failed) => had_failures |= failed,
            Err(e) => return Err(e),
        }
    }

    if had_failures {
        info!("Completed with some failures (--skip-failures enabled)");
    }

    Ok(())
}
