use std::path::{Path, PathBuf};

use futures::StreamExt;
use log::{debug, error, info, trace};
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
    let file = File::open(path)
        .await
        .map_err(|e| Error::Download(format!("Failed to open URL list file: {}", e)))?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut urls = Vec::new();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim();
        if !line.is_empty() && !line.starts_with('#') {
            urls.push(line.to_string());
        }
    }

    if urls.is_empty() {
        return Err(Error::Download("No valid URLs found in file".to_string()));
    }

    info!("Found {} URLs to process", urls.len());
    Ok(urls)
}

fn build_download_config(args: &HasherDownloadArgs) -> DownloadConfig {
    DownloadConfig {
        retry_count: args.hash_options.retry_count,
        retry_delay: std::time::Duration::from_secs(args.hash_options.retry_delay as u64),
        compress: args.hash_options.compress,
        compression_level: args.hash_options.compression_level,
        no_clobber: args.no_clobber,
    }
}

fn build_result_json(result: &DownloadResult, pretty: bool) -> String {
    let result_type = match (&result.success, &result.error) {
        (true, Some(e)) if e == "File exists, skipping download" => "download_skipped",
        (true, _) => "download_success",
        (false, _) => "download_failure",
    };

    let json = serde_json::json!({
        "url": result.url,
        "destination": result.path,
        "size": result.size,
        "error": result.error.clone(),
        "type": result_type
    });

    if pretty {
        serde_json::to_string_pretty(&json)
    } else {
        serde_json::to_string(&json)
    }
    .unwrap()
}

async fn process_download_result(
    result: DownloadResult,
    args: &HasherDownloadArgs,
    config: &Config,
) -> Result<bool, Error> {
    trace!("Processing download result for {}", result.url);

    // Output JSON unless SQL-only is explicitly set
    if !args.hash_options.sql_only || args.hash_options.json_only {
        println!(
            "{}",
            build_result_json(&result, args.hash_options.pretty_json)
        );
    }

    match (&result.success, &result.error) {
        // Skip further processing for no-clobber skips
        (true, Some(e)) if e == "File exists, skipping download" => Ok(false),

        // Process successful downloads
        (true, _) => match crate::output::process_single_file(
            &result.path,
            config,
            &args.hash_options,
            &mut None,
        )
        .await
        {
            Ok(()) => Ok(false),
            Err(e) if !args.hash_options.fail_fast => {
                println!(
                    "{}",
                    serde_json::json!({
                        "url": result.url,
                        "destination": result.path,
                        "error": e.to_string(),
                        "type": "hash_failure"
                    })
                );
                Ok(true)
            }
            Err(e) => Err(e),
        },

        // Handle failures
        (false, _) if !args.hash_options.fail_fast && args.hash_options.silent_failures => {
            error!("Failed to download {}: Unknown error", result.url);
            Ok(true)
        }
        (false, _) if !args.hash_options.fail_fast => {
            Err(Error::Download(format!(
                "Failed to download {}: Unknown error",
                result.url
            )))
        }
        (false, Some(e)) => Err(Error::Download(format!(
            "Failed to download {}: {}",
            result.url, e
        ))),
        (false, None) => Err(Error::Download(format!(
            "Failed to download {}: Unknown error",
            result.url
        ))),
    }
}

pub async fn execute(args: HasherDownloadArgs, config: &Config) -> Result<(), Error> {
    debug!("Executing download command");

    // Get URLs from file or command line
    let urls = if Path::new(&args.source).is_file() {
        info!("Reading URLs from file: {}", args.source.display());
        read_url_list(Path::new(&args.source)).await?
    } else {
        info!(
            "Using single URL from command line: {}",
            args.source.display()
        );
        vec![args.source.to_string_lossy().to_string()]
    };

    if urls.is_empty() {
        return Err(Error::Download("No URLs to process".to_string()));
    }
    info!("Processing {} URL(s)", urls.len());
    trace!("URLs to process: {:?}", urls);

    let should_compress = args.hash_options.compress;
    let compression_level = args.hash_options.compression_level;
    let downloader = Downloader::new(build_download_config(&args));
    let mut stream = downloader
        .download_from_list(urls, &args.destination, move |url| {
            let base_path = construct_download_path(url, Path::new(""))
                .unwrap_or_else(|_| Path::new("").join(sanitize_filename(url)));

            if !should_compress {
                return base_path;
            }

            let compressor = compression::get_compressor(CompressionType::Gzip, compression_level);
            base_path.with_extension(format!(
                "{}{}",
                base_path.extension().unwrap_or_default().to_string_lossy(),
                compressor.extension()
            ))
        })
        .await;

    let mut had_failures = false;
    while let Some(result) = stream.next().await {
        match process_download_result(result, &args, config).await {
            Ok(true) => had_failures = true,
            Ok(false) => (),
            Err(_) if !args.hash_options.fail_fast => had_failures = true,
            Err(e) => return Err(e),
        }
    }

    if had_failures {
        info!("Completed with some failures");
    }

    Ok(())
}
