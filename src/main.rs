use chrono::offset::Utc;
use std::io::Write;
use std::time::Instant;
use std::{path::PathBuf, process::exit};

use clap::Parser;
use log::{error, warn, info};
use tokio;

mod configuration;
mod hasher;

macro_rules! startlogging {
    ($config_args:ident) => {
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
            .filter_level($config_args.verbose.log_level_filter())
            .init();
    };
}

#[tokio::main]
async fn main() {
    let start_time = Instant::now();
    let args = configuration::Args::parse();

    startlogging!(args);

    let config = configuration::get_config(&args);
    let input_path = PathBuf::from(&args.input_path);

    // Only sqlite databases are supported at the moment
    if args.sql_out && !config.database.db_string.starts_with("sqlite") {
        error!("Non-sqlite databases are not implemented yet!");
        exit(1);
    }

    if !args.sql_out && !args.json_out {
        warn!("No output method selected! Hashed results will not go anywhere!");
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

    info!("Execution took: {:.2?}.", start_time.elapsed());
}
