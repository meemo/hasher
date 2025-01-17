use chrono::Utc;
use std::io::Write;
use std::process::exit;

use clap::Parser;
use clap_verbosity_flag::Verbosity;
use log::{error, warn};

use crate::configuration::{HasherCli, HasherCommand};

mod commands;
mod compression;
mod configuration;
mod database;
mod downloader;
mod output;
mod utils;

fn setup_logging<T: clap_verbosity_flag::LogLevel>(verbose: &Verbosity<T>) {
    env_logger::builder()
        .format(|buf, record| {
            writeln!(
                buf,
                "[{}][{}] {}",
                Utc::now().timestamp(),
                record.level(),
                record.args()
            )
        })
        .filter_level(verbose.log_level_filter())
        .init();
}

#[tokio::main]
async fn main() {
    let start_time = std::time::Instant::now();
    let args = HasherCli::parse();

    let (hash_options, config_file) = match &args.command {
        HasherCommand::Hash(args) => (&args.hash_options, &args.hash_options.config_file),
        HasherCommand::Copy(args) => (&args.hash_options, &args.hash_options.config_file),
        HasherCommand::Verify(args) => (&args.hash_options, &args.hash_options.config_file),
        HasherCommand::Download(args) => (&args.hash_options, &args.hash_options.config_file),
    };

    setup_logging(&hash_options.verbose);

    let config = match configuration::get_config(config_file, hash_options.db_path.as_deref()) {
        Ok(config) => config,
        Err(e) => {
            error!("Configuration error: {}", e);
            exit(1);
        }
    };

    let should_close_wal = match &args.command {
        HasherCommand::Hash(args) => {
            if !args.hash_options.json_only {
                // Initialize DB if we're not JSON-only
                if let Err(e) = database::init_database(
                    &config.database.db_string,
                    &config.database.table_name,
                    args.hash_options.use_wal,
                )
                .await
                {
                    error!("Database initialization error: {}", e);
                    exit(1);
                }
            }

            if args.hash_options.sql_only && args.hash_options.json_only {
                warn!("Both --sql-only and --json-only specified, defaulting to both outputs");
            }

            if !args.hash_options.dry_run
                && args.hash_options.sql_only
                && args.hash_options.json_only
            {
                warn!("No output method available! Remove --sql-only or --json-only (see --help)");
                exit(1);
            }

            !args.hash_options.json_only && args.hash_options.use_wal
        }
        _ => false,
    };

    let result = match args.command {
        HasherCommand::Hash(args) => commands::hash::execute(args, &config).await,
        HasherCommand::Copy(args) => commands::copy::execute(args, &config).await,
        HasherCommand::Verify(args) => commands::verify::execute(args, &config).await,
        HasherCommand::Download(args) => commands::download::execute(args, &config).await,
    };

    if let Err(e) = result {
        error!("Fatal error: {}", e);
        exit(1);
    }

    if should_close_wal {
        database::close_database(&config.database.db_string).await;
    }

    log::info!("Execution took: {:.2?}", start_time.elapsed());
}
