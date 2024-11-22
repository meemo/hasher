use std::io::Read;
use std::path::Path;
use std::time::Duration;

use log::{error, info};
use serde_json::json;
use sqlx::{query_builder::QueryBuilder, Connection, SqliteConnection};
use walkdir::WalkDir;

use crate::configuration::{Config, HasherOptions};
use crate::compression::{self, CompressionAlgorithm};
use crate::utils::Error;
use hasher::{HashConfig, Hasher};

const MAX_DB_RETRIES: u32 = 3;
const DB_RETRY_DELAY: Duration = Duration::from_millis(100);

async fn insert_hash_to_db(
    config: &Config,
    file_path: &Path,
    size: usize,
    hashes: &[(&str, Vec<u8>)],
    db_conn: &mut SqliteConnection,
) -> Result<(), Error> {
    let mut retries = 0;
    loop {
        let mut query_builder: QueryBuilder<sqlx::Sqlite> = QueryBuilder::new("INSERT INTO ");
        query_builder.push(config.database.table_name.clone());

        let mut sep = query_builder.separated(", ");
        sep.push_unseparated(" (");
        sep.push("file_path");
        sep.push("file_size");
        for (hash_name, _) in hashes {
            sep.push(*hash_name);
        }

        let mut sep = query_builder.separated(", ");
        sep.push_unseparated(") VALUES (");
        sep.push_bind(file_path.display().to_string());
        sep.push_bind(size as f64);
        for (_, hash_data) in hashes {
            sep.push_bind(hash_data.as_slice());
        }
        sep.push_unseparated(");");

        match query_builder.build().execute(&mut *db_conn).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                if let Some(db_err) = e.as_database_error() {
                    if db_err.code().as_deref() == Some("SQLITE_BUSY") && retries < MAX_DB_RETRIES {
                        retries += 1;
                        tokio::time::sleep(DB_RETRY_DELAY).await;
                        continue;
                    }
                }
                return Err(Error::from(e));
            }
        }
    }
}

fn output_json(file_path: &Path, file_size: usize, hashes: &[(&str, Vec<u8>)], pretty: bool) {
    let mut hash_map = serde_json::Map::new();
    hash_map.insert(
        "file_path".to_string(),
        json!(file_path.display().to_string()),
    );
    hash_map.insert("file_size".to_string(), json!(file_size));

    for (hash_name, hash_data) in hashes {
        hash_map.insert(hash_name.to_string(), json!(hex::encode(hash_data)));
    }

    let output = if pretty {
        serde_json::to_string_pretty(&hash_map)
    } else {
        serde_json::to_string(&hash_map)
    }
    .unwrap();

    println!("{}", output);
}

fn log_hash_results(file_path: &Path, hashes: &[(&str, Vec<u8>)]) {
    info!("Successfully hashed {}", file_path.display());
    for (name, hash) in hashes {
        info!("{}: {}", name, hex::encode(hash));
    }
}

pub async fn process_single_file(
    file_path: &Path,
    config: &Config,
    args: &HasherOptions,
    db_conn: &mut Option<SqliteConnection>,
) -> Result<(), Error> {
    let mut hasher = Hasher::new(HashConfig::from(&config.hashes));
    let compressor = compression::get_compressor(compression::CompressionType::Gzip, 6);

    let process_file = |data: &[u8], _path: &Path| -> Result<(usize, Vec<(&str, Vec<u8>)>), Error> {
        let mut hasher = Hasher::new(HashConfig::from(&config.hashes));
        hasher.hash_single_buffer(data)
            .map(|hashes| (data.len(), hashes))
            .map_err(Error::from)
    };

    let result = if args.decompress && compressor.is_compressed_path(file_path) {
        let compressed_data = tokio::fs::read(file_path).await?;
        let decompressed = compression::decompress_bytes(&compressed_data, compression::CompressionType::Gzip)?;

        if args.hash_both {
            let (comp_size, comp_hashes) = process_file(&compressed_data, file_path)?;
            let (decomp_size, decomp_hashes) = process_file(&decompressed, file_path)?;

            // Output both results
            if !args.dry_run {
                let do_sql = !args.json_only;
                let do_json = !args.sql_only;

                if do_sql {
                    if let Some(conn) = db_conn {
                        insert_hash_to_db(config, file_path, comp_size, &comp_hashes, conn).await?;
                        let decomp_path = file_path.with_extension("");
                        insert_hash_to_db(config, &decomp_path, decomp_size, &decomp_hashes, conn).await?;
                    }
                }

                if do_json {
                    output_json(file_path, comp_size, &comp_hashes, args.pretty_json);
                    let decomp_path = file_path.with_extension("");
                    output_json(&decomp_path, decomp_size, &decomp_hashes, args.pretty_json);
                }
            }
            Ok(())
        } else {
            let (size, hashes) = process_file(&decompressed, file_path)?;
            log_hash_results(file_path, &hashes);

            if !args.dry_run {
                let do_sql = !args.json_only;
                let do_json = !args.sql_only;

                if do_sql {
                    if let Some(conn) = db_conn {
                        insert_hash_to_db(config, file_path, size, &hashes, conn).await?;
                    }
                }

                if do_json {
                    output_json(file_path, size, &hashes, args.pretty_json);
                }
            }
            Ok(())
        }
    } else {
        match hasher.hash_file(file_path) {
            Ok((file_size, hashes)) => {
                log_hash_results(file_path, &hashes);

                if !args.dry_run {
                    let do_sql = !args.json_only;
                    let do_json = !args.sql_only;

                    if do_sql {
                        if let Some(conn) = db_conn {
                            insert_hash_to_db(config, file_path, file_size, &hashes, conn).await?;
                        }
                    }

                    if do_json {
                        output_json(file_path, file_size, &hashes, args.pretty_json);
                    }
                }
                Ok(())
            }
            Err(e) => Err(Error::from(e))
        }
    };

    if let Err(e) = result {
        let err_msg = format!("Failed to hash {}: {}", file_path.display(), e);
        if args.continue_on_error {
            error!("{}", err_msg);
            Ok(())
        } else {
            Err(e)
        }
    } else {
        result
    }
}

pub async fn process_stdin(
    config: &Config,
    file_path: &str,
    conn: &mut SqliteConnection,
) -> Result<(), Error> {
    let mut buffer = Vec::new();
    std::io::stdin().read_to_end(&mut buffer)?;

    let mut hasher = Hasher::new(HashConfig::from(&config.hashes));
    let hashes = hasher.hash_single_buffer(&buffer)?;

    insert_hash_to_db(config, Path::new(file_path), buffer.len(), &hashes, conn).await?;

    Ok(())
}

pub async fn process_directory(
    path_to_hash: &Path,
    args: &HasherOptions,
    config: &Config,
) -> Result<(), Error> {
    let mut db_conn = if !args.json_only {
        Some(SqliteConnection::connect(&config.database.db_string).await?)
    } else {
        None
    };

    let mut file_count = 0;
    let walker = WalkDir::new(path_to_hash)
        .min_depth(0)
        .max_depth(args.max_depth)
        .follow_links(!args.no_follow_symlinks)
        .contents_first(!args.breadth_first)
        .sort_by_file_name();

    for entry in walker {
        if let Ok(entry) = entry {
            if !entry.path().is_dir() {
                file_count += 1;
                process_single_file(entry.path(), config, args, &mut db_conn).await?;
            }
        }
    }

    info!(
        "Successfully processed {} files at path: {}",
        file_count,
        path_to_hash.display()
    );

    Ok(())
}
