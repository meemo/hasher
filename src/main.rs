use chrono::offset::Utc;
use std::io::Write;
use std::time::Instant;
use std::{path::PathBuf, process::exit};

use clap::Parser;
use log::{error, info};

mod configuration;
mod hasher;

fn main() {
    let start_time = Instant::now();
    let config_args = configuration::HasherArgs::parse();

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
        .filter_level(config_args.verbose.log_level_filter())
        .init();

    let config = configuration::get_config(&config_args);
    let input_path = PathBuf::from(&config_args.input_path);

    if let Ok(_) = hasher::hash_dir(input_path.as_path(), &config_args, &config) {
        info!("Execution took: {:.2?}.", start_time.elapsed());
    } else {
        error!("Failure while hashing directory {}", input_path.display());
        exit(1);
    }
}
