use chrono::offset::Utc;
use std::{io::Write, time::Instant, path::PathBuf, process::exit};

use clap::Parser;
use log::{error, warn, info};
use tokio;

mod utils;
mod configuration;
mod hasher;
mod database;

#[tokio::main]
async fn main() {
    let start_time = Instant::now();
    let args = configuration::Args::parse();

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
        .filter_level(args.verbose.log_level_filter())
        .init();

    let config = configuration::get_config(&args);
    let input_path = PathBuf::from(&args.input_path);

    if args.sql_out {
        if config.database.db_string.starts_with("sqlite://") {
            database::init_database(&config.database.db_string, &config.database.table_name, args.use_wal)
                .await
                .expect("Failed to initialize the database!");
        } else {
            // Only sqlite databases are supported at the moment
            error!("Non-sqlite databases are not supported yet!");
            exit(1);
        }
    }

    // Dry runs ignore sql_out and json_out options
    if !args.dry_run && !args.sql_out && !args.json_out {
        warn!("No output method selected! Use --sql-out or --json-out (see --help).");
        exit(1);
    }


    if args.stdin {
        // Hash the data provided in stdin
        hasher::hash_stdin(&config, &args.input_path)
            .await
            .expect("Failure while hashing from stdin!");
    } else {
        // Hash the file at the given path
        hasher::hash_dir(input_path.as_path(), &args, &config)
            .await
            .expect("Failure while hashing directory!");
    }


    if args.sql_out && args.use_wal {
        database::close_database(&config.database.db_string).await;
    }

    info!("Execution took: {:.2?}.", start_time.elapsed());
}
