use std::process::exit;
use std::io::Write;
use chrono::Utc;

use clap::Parser;
use log::{error, warn};

mod utils;
mod configuration;
mod output;
mod database;
mod commands;

use configuration::{HasherArgs, HasherCommand, HasherOptions, Config, get_config};
use utils::Error;

fn setup_logging(options: &HasherOptions) {
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
        .filter_level(options.verbose.log_level_filter())
        .init();
}

async fn init_database(options: &HasherOptions, config: &Config) -> Result<(), Error> {
    if !config.database.db_string.starts_with("sqlite://") {
        return Err(Error::Config("Non-sqlite databases are not supported".into()));
    }

    database::init_database(
        &config.database.db_string,
        &config.database.table_name,
        options.use_wal
    ).await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let start_time = std::time::Instant::now();
    let args = HasherArgs::parse();

    // Clone the options reference for later use
    let mut options = {
        let cmd_options = match &args.command {
            HasherCommand::Hash(args) => &args.hash_options,
            HasherCommand::Copy(args) => &args.hash_options,
            HasherCommand::Verify(args) => &args.hash_options,
        };
        cmd_options.clone()
    };

    setup_logging(&options);

    let config = match get_config(&options) {
        Ok(config) => config,
        Err(e) => {
            error!("Configuration error: {}", e);
            exit(1);
        }
    };

    if options.sql_out {
        if let Err(e) = init_database(&options, &config).await {
            error!("Database initialization error: {}", e);
            exit(1);
        }
    }

    if !options.dry_run && !options.sql_out && !options.json_out {
        warn!("No output method selected! Use --sql-out or --json-out (see --help)");
        exit(1);
    }

    if !options.dry_run && !options.sql_out && !options.json_out {
        // Only warn if we're hashing, copy/verify don't need output methods
        if matches!(args.command, HasherCommand::Hash(_)) {
            // Default to JSON output
            warn!("No output method selected, defaulting to JSON output");
            options.json_out = true;
        }
    }

    let result = match args.command {
        HasherCommand::Hash(args) => commands::hash::execute(args, &config).await,
        HasherCommand::Copy(args) => commands::copy::execute(args, &config).await,
        HasherCommand::Verify(args) => commands::verify::execute(args, &config).await,
    };

    if let Err(e) = result {
        error!("Fatal error: {}", e);
        exit(1);
    }

    if options.sql_out && options.use_wal {
        database::close_database(&config.database.db_string).await;
    }

    log::info!("Execution took: {:.2?}", start_time.elapsed());
}
