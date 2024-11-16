use std::path::PathBuf;
use sqlx::{SqliteConnection, Connection};

use crate::configuration::{HasherHashArgs, Config};
use crate::output;
use crate::utils::Error;

pub async fn execute(args: HasherHashArgs, config: &Config) -> Result<(), Error> {
    let input_path = args.source.unwrap_or_else(|| PathBuf::from("."));

    if args.hash_options.stdin {
        let mut conn = SqliteConnection::connect(&config.database.db_string)
            .await
            .map_err(Error::Database)?;
        output::process_stdin(&config, &input_path.to_string_lossy(), &mut conn).await
    } else {
        output::process_directory(&input_path, &args.hash_options, &config).await
    }
}
