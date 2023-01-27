use chrono::offset::Utc;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

use log::info;

mod configuration;
mod hasher;

use configuration::get_config;
use hasher::hash_dir;

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
    let config = get_config();
    let input_path = PathBuf::from(&config.input_path);

    hash_dir(input_path.as_path(), &config).expect("Failure while hashing directory!");

    info!("Execution took: {:.2?}.", start_time.elapsed());
}
