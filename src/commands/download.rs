use log::info;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::configuration::{Config, HasherDownloadArgs};
use crate::downloader::{sanitize_filename, DownloadConfig, Downloader};
use crate::utils::Error;

pub async fn execute(args: HasherDownloadArgs, config: &Config) -> Result<(), Error> {
    let downloader = Downloader::new(DownloadConfig::default());

    let urls = if args.source.exists() {
        let file = File::open(&args.source).await?;
        let reader = BufReader::new(file);
        let mut lines = reader.lines();
        let mut urls = Vec::new();

        while let Some(line) = lines.next_line().await? {
            let line = line.trim();
            if !line.is_empty() {
                urls.push(line.to_string());
            }
        }
        urls
    } else {
        vec![args.source.to_string_lossy().to_string()]
    };

    let results = downloader
        .download_from_list(urls, &args.destination, |url| sanitize_filename(url))
        .await;

    for result in results {
        if result.success {
            info!(
                "Successfully downloaded {} to {}",
                result.url,
                result.path.display()
            );

            if !args.hash_options.dry_run {
                crate::output::process_single_file(
                    &result.path,
                    config,
                    &args.hash_options,
                    &mut None,
                )
                .await?;
            }
        } else if !args.hash_options.continue_on_error {
            return Err(Error::Download(format!(
                "Failed to download {}: {}",
                result.url,
                result.error.unwrap_or_else(|| "Unknown error".to_string())
            )));
        }
    }

    Ok(())
}
