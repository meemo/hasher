use std::path::PathBuf;
use std::time::Instant;

use log::{error, info};

mod hasher;
mod configuration;

use hasher::hash_dir;
use configuration::get_config;

fn main() {
    env_logger::init();

    let start_time = Instant::now();

    let config = get_config();

    let input_path = PathBuf::from(&config.path);

    if let Ok(_) = hash_dir(input_path.as_path(), &config) {
    } else {
        error!("Failure while hashing directory!");
    }

    info!("Execution took: {:.2?}.", start_time.elapsed());
}
