use sqlx::{Connection, SqliteConnection};
use std::path::PathBuf;

use crate::configuration::{Config, HasherHashArgs};
use crate::output;
use crate::utils::Error;

pub async fn execute(args: HasherHashArgs, config: &Config) -> Result<Option<serde_json::Map<String, serde_json::Value>>, Error> {
    let input_path = args.source.unwrap_or_else(|| PathBuf::from("."));

    if args.hash_options.stdin {
        let mut conn = if !args.hash_options.json_only {
            Some(
                SqliteConnection::connect(&config.database.db_string)
                    .await
                    .map_err(Error::from)?,
            )
        } else {
            None
        };

        output::process_stdin(
            &config,
            &input_path.to_string_lossy(),
            &mut conn,
            &args.hash_options,
        )
        .await
    } else {
        output::process_directory(&input_path, &args.hash_options, &config).await
    }
}
