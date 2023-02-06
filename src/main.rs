use chrono::offset::Utc;
use std::path::PathBuf;
use std::time::Instant;
use std::{io::Write, process::exit};

use log::info;

mod configuration;
mod hasher;

use configuration::get_args;
use hasher::hash_dir;

use crate::configuration::{get_config, write_config_template};

fn main() {
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
        .init();

    let start_time = Instant::now();
    let config_args = get_args();

    if config_args.write_config_template {
        write_config_template();
        exit(0);
    }

    let config = get_config(&config_args);
    let input_path = PathBuf::from(&config_args.input_path);

    hash_dir(input_path.as_path(), &config_args, &config)
        .expect("Failure while hashing directory!");

    info!("Execution took: {:.2?}.", start_time.elapsed());
}
