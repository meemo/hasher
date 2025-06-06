use std::io::Read;
use std::path::Path;

use log::{error, info};
use serde_json::json;
use sqlx::{Connection, SqliteConnection};
use walkdir::WalkDir;

use crate::compression::{self, CompressionAlgorithm};
use crate::configuration::{Config, HasherOptions};
use crate::database::insert_single_hash;
use crate::utils::Error;
use hasher::{HashConfig, Hasher};

fn build_hash_json(
    file_path: &Path,
    file_size: usize,
    hashes: &[(&str, Vec<u8>)],
) -> serde_json::Map<String, serde_json::Value> {
    let mut hash_map = serde_json::Map::new();
    hash_map.insert(
        "file_path".to_string(),
        json!(file_path.display().to_string()),
    );
    hash_map.insert("file_size".to_string(), json!(file_size));

    for (hash_name, hash_data) in hashes {
        hash_map.insert(hash_name.to_string(), json!(hex::encode(hash_data)));
    }

    hash_map
}

fn output_json(
    file_path: &Path,
    file_size: usize,
    hashes: &[(&str, Vec<u8>)],
    pretty: bool,
) -> Option<serde_json::Map<String, serde_json::Value>> {
    let hash_map = build_hash_json(file_path, file_size, hashes);

    let output = if pretty {
        serde_json::to_string_pretty(&hash_map)
    } else {
        serde_json::to_string(&hash_map)
    }
    .unwrap();

    println!("{}", output);
    Some(hash_map)
}

fn log_hash_results(file_path: &Path, hashes: &[(&str, Vec<u8>)]) {
    info!("Successfully hashed {}", file_path.display());
    for (name, hash) in hashes {
        info!("{}: {}", name, hex::encode(hash));
    }
}

async fn store_hash_results(
    config: &Config,
    file_path: &Path,
    size: usize,
    hashes: &[(&str, Vec<u8>)],
    args: &HasherOptions,
    db_conn: &mut Option<SqliteConnection>,
) -> Result<Option<serde_json::Map<String, serde_json::Value>>, Error> {
    if args.dry_run {
        return Ok(None);
    }

    let do_sql = !args.json_only;
    let do_json = !args.sql_only;

    if do_sql {
        if let Some(conn) = db_conn {
            insert_single_hash(config, file_path, size, hashes, conn).await?;
        }
    }

    if do_json {
        Ok(output_json(file_path, size, hashes, args.pretty_json))
    } else {
        Ok(None)
    }
}

async fn process_compressed_file(
    file_path: &Path,
    config: &Config,
    args: &HasherOptions,
    db_conn: &mut Option<SqliteConnection>,
) -> Result<Option<serde_json::Map<String, serde_json::Value>>, Error> {
    let compressor =
        compression::get_compressor(compression::CompressionType::Gzip, args.compression_level);

    // Read or compress the file data
    let compressed_data = if compressor.is_compressed_path(file_path) {
        tokio::fs::read(file_path).await?
    } else {
        let data = tokio::fs::read(file_path).await?;
        compression::compress_bytes(
            &data,
            compression::CompressionType::Gzip,
            args.compression_level,
        )
        .map_err(Error::from)?
    };

    // Create a hasher with the config
    let mut hasher = Hasher::new(HashConfig::from(&config.hashes));

    if args.hash_both {
        // Hash both compressed and decompressed states
        let comp_hashes = hasher.hash_single_buffer(&compressed_data)?;
        let comp_size = compressed_data.len();

        let decompressed =
            compression::decompress_bytes(&compressed_data, compression::CompressionType::Gzip)
                .map_err(Error::from)?;
        let decomp_hashes = hasher.hash_single_buffer(&decompressed)?;
        let decomp_size = decompressed.len();

        if !args.dry_run {
            let do_sql = !args.json_only;
            let do_json = !args.sql_only;

            if do_sql {
                if let Some(conn) = db_conn {
                    insert_single_hash(config, file_path, comp_size, &comp_hashes, conn).await?;
                    let decomp_path = file_path.with_extension("");
                    insert_single_hash(config, &decomp_path, decomp_size, &decomp_hashes, conn)
                        .await?;
                }
            }

            if do_json {
                let hash_info = output_json(file_path, comp_size, &comp_hashes, args.pretty_json);
                let decomp_path = file_path.with_extension("");
                output_json(&decomp_path, decomp_size, &decomp_hashes, args.pretty_json);
                Ok(hash_info)
            } else {
                Ok(None)
            }
        } else {
            Ok(None)
        }
    } else if args.decompress || args.hash_uncompressed {
        // Only hash decompressed state
        let decompressed =
            compression::decompress_bytes(&compressed_data, compression::CompressionType::Gzip)
                .map_err(Error::from)?;
        let hashes = hasher.hash_single_buffer(&decompressed)?;
        let size = decompressed.len();

        log_hash_results(file_path, &hashes);
        store_hash_results(config, file_path, size, &hashes, args, db_conn).await
    } else {
        // Only hash compressed state
        let hashes = hasher.hash_single_buffer(&compressed_data)?;
        let size = compressed_data.len();

        log_hash_results(file_path, &hashes);
        store_hash_results(config, file_path, size, &hashes, args, db_conn).await
    }
}

async fn process_uncompressed_file(
    file_path: &Path,
    config: &Config,
    args: &HasherOptions,
    db_conn: &mut Option<SqliteConnection>,
) -> Result<Option<serde_json::Map<String, serde_json::Value>>, Error> {
    let mut hasher = Hasher::new(HashConfig::from(&config.hashes));
    let (file_size, hashes) = hasher.hash_file(file_path)?;

    log_hash_results(file_path, &hashes);
    store_hash_results(config, file_path, file_size, &hashes, args, db_conn).await
}

pub async fn process_single_file(
    file_path: &Path,
    config: &Config,
    args: &HasherOptions,
    db_conn: &mut Option<SqliteConnection>,
) -> Result<Option<serde_json::Map<String, serde_json::Value>>, Error> {
    let compressor =
        compression::get_compressor(compression::CompressionType::Gzip, args.compression_level);

    let result = if compressor.is_compressed_path(file_path) || args.hash_compressed {
        process_compressed_file(file_path, config, args, db_conn).await
    } else {
        process_uncompressed_file(file_path, config, args, db_conn).await
    };

    // Handle errors according to fail_fast and silent_failures settings
    match result {
        Err(e) if !args.fail_fast => {
            if !args.silent_failures {
                error!("Failed to hash {}: {}", file_path.display(), e);
            }
            Ok(None)
        }
        other => other,
    }
}

pub async fn process_stdin(
    config: &Config,
    file_path: &str,
    conn: &mut Option<SqliteConnection>,
    args: &HasherOptions,
) -> Result<Option<serde_json::Map<String, serde_json::Value>>, Error> {
    let mut buffer = Vec::new();
    std::io::stdin().read_to_end(&mut buffer)?;

    let mut hasher = Hasher::new(HashConfig::from(&config.hashes));
    let hashes = hasher.hash_single_buffer(&buffer)?;

    let do_sql = !args.json_only;
    let do_json = !args.sql_only;

    if do_sql {
        if let Some(conn) = conn {
            insert_single_hash(config, Path::new(file_path), buffer.len(), &hashes, conn).await?;
        }
    }

    if do_json {
        Ok(output_json(
            Path::new(file_path),
            buffer.len(),
            &hashes,
            args.pretty_json,
        ))
    } else {
        Ok(None)
    }
}

pub async fn process_directory(
    path_to_hash: &Path,
    args: &HasherOptions,
    config: &Config,
) -> Result<Option<serde_json::Map<String, serde_json::Value>>, Error> {
    let mut db_conn = if !args.json_only {
        Some(SqliteConnection::connect(&config.database.db_string).await?)
    } else {
        None
    };

    let mut file_count = 0;
    let mut last_hash_info = None;
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
                last_hash_info =
                    process_single_file(entry.path(), config, args, &mut db_conn).await?;
            }
        }
    }

    info!(
        "Successfully processed {} files at path: {}",
        file_count,
        path_to_hash.display()
    );

    Ok(last_hash_info)
}
