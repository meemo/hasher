use chrono::offset::Utc;
use std::io::Write;
use std::time::Instant;
use std::{path::PathBuf, process::exit};

use clap::Parser;
use log::{error, info};
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
    let config_args = configuration::Args::parse();

    startlogging!(config_args);

    let config = configuration::get_config(&config_args);
    let input_path = PathBuf::from(&config_args.input_path);

    if config_args.stdin {
        // Hash the data provided in stdin
        if let Ok(_) = hasher::hash_stdin(&config.hashes, &config_args.input_path) {
            // do nothing
        } else {
            error!("Failure while hashing from stdin!");
            exit(1);
        }
    } else {
        // Hash the file at the given path
        if let Ok(_) = hasher::hash_dir(input_path.as_path(), &config_args, &config).await {
            // do nothing
        } else {
            error!("Failure while hashing directory {}", input_path.display());
            exit(1);
        }
    }

    info!("Execution took: {:.2?}.", start_time.elapsed());
}
