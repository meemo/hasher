use std::path::PathBuf;
use std::process::exit;
use std::io::Write;
use chrono::Utc;

use clap::Parser;
use log::{error, warn, info};
use sqlx::{Connection, SqliteConnection};

mod utils;
mod configuration;
mod output;
mod database;

#[tokio::main]
async fn main() {
    let start_time = std::time::Instant::now();
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
                .expect("Failed to initialize database!");
        } else {
            error!("Non-sqlite databases are not supported!");
            exit(1);
        }
    }

    if !args.dry_run && !args.sql_out && !args.json_out {
        warn!("No output method selected! Use --sql-out or --json-out (see --help)");
        exit(1);
    }

    let result = if args.stdin {
        let mut conn = SqliteConnection::connect(&config.database.db_string)
            .await
            .expect("Failed to connect to database");
        output::process_stdin(&config, &args.input_path, &mut conn).await
    } else {
        output::process_directory(&input_path, &args, &config).await
    };

    if let Err(e) = result {
        error!("Error during processing: {:?}", e);
        exit(1);
    }

    if args.sql_out && args.use_wal {
        database::close_database(&config.database.db_string).await;
    }

    info!("Execution took: {:.2?}", start_time.elapsed());
}
