use serde_json::json;
use sqlx::Connection;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter};
use std::path::Path;

use log::{error, info};
use walkdir::WalkDir;

use crate::compression::{self, CompressionAlgorithm};
use crate::configuration::{Config, HasherCopyArgs};
use crate::database::insert_single_hash;
use crate::utils::Error;
use hasher::{HashConfig, Hasher};

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

async fn process_hash_results(
    path: &Path,
    file_size: usize,
    hashes: &[(&str, Vec<u8>)],
    args: &HasherCopyArgs,
    config: &Config,
    db_conn: &mut Option<sqlx::SqliteConnection>,
) -> Result<(), Error> {
    let do_sql = !args.hash_options.json_only;
    let do_json = !args.hash_options.sql_only;

    if do_sql {
        if let Some(conn) = db_conn {
            insert_single_hash(config, path, file_size, hashes, conn).await?;
        }
    }

    if do_json {
        output_json(path, file_size, hashes, args.hash_options.pretty_json);
    }

    Ok(())
}

async fn copy_and_hash_file(
    source: &Path,
    dest: &Path,
    args: &HasherCopyArgs,
    config: &Config,
    db_conn: &mut Option<sqlx::SqliteConnection>,
) -> Result<(), Error> {
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    let final_dest = if args.hash_options.compress {
        let compressor = compression::get_compressor(
            compression::CompressionType::Gzip,
            args.hash_options.compression_level,
        );
        dest.with_extension(format!(
            "{}{}",
            dest.extension().unwrap_or_default().to_string_lossy(),
            compressor.extension()
        ))
    } else {
        dest.to_path_buf()
    };

    if !args.hash_options.dry_run {
        if args.hash_options.compress {
            let source_data = fs::read(source)?;
            let compressed = compression::compress_bytes(
                &source_data,
                compression::CompressionType::Gzip,
                args.hash_options.compression_level,
            )
            .map_err(Error::IO)?;
            fs::write(&final_dest, compressed)?;
        } else {
            let reader = BufReader::new(File::open(source)?);
            let writer = BufWriter::new(File::create(&final_dest)?);
            io::copy(&mut BufReader::new(reader), &mut BufWriter::new(writer))?;
        }

        let path_to_hash = if args.store_source_path {
            source
        } else if args.hash_options.hash_compressed {
            &final_dest
        } else {
            source
        };

        let mut hasher = Hasher::new(HashConfig::from(&config.hashes));
        match hasher.hash_file(path_to_hash) {
            Ok((file_size, hashes)) => {
                process_hash_results(path_to_hash, file_size, &hashes, args, config, db_conn)
                    .await?;
            }
            Err(e) => {
                if args.hash_options.continue_on_error {
                    error!("Failed to hash {}: {}", path_to_hash.display(), e);
                } else {
                    return Err(Error::from(e));
                }
            }
        }
    }

    Ok(())
}

async fn copy_directory(
    base_source: &Path,
    base_dest: &Path,
    args: &HasherCopyArgs,
    config: &Config,
    db_conn: &mut Option<sqlx::SqliteConnection>,
) -> Result<u64, Error> {
    let mut copied_count = 0;

    for entry in WalkDir::new(base_source)
        .min_depth(0)
        .max_depth(args.hash_options.max_depth)
        .follow_links(!args.hash_options.no_follow_symlinks)
        .contents_first(!args.hash_options.breadth_first)
        .sort_by_file_name()
    {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let rel_path = path
                .strip_prefix(base_source)
                .map_err(|_| Error::Config("Failed to strip prefix".into()))?;
            let dest_path = base_dest.join(rel_path);

            if let Err(e) = copy_and_hash_file(path, &dest_path, args, config, db_conn).await {
                let err_msg = format!("Failed to copy {}: {}", path.display(), e);
                if args.hash_options.continue_on_error {
                    error!("{}", err_msg);
                    continue;
                }
                return Err(e);
            }
            copied_count += 1;
        }
    }

    Ok(copied_count)
}

pub async fn execute(args: HasherCopyArgs, config: &Config) -> Result<(), Error> {
    let source = &args.source;
    let dest = &args.destination;

    if !source.exists() {
        return Err(Error::Config("Source path does not exist".into()));
    }

    let mut db_conn = if !args.hash_options.json_only {
        Some(sqlx::SqliteConnection::connect(&config.database.db_string).await?)
    } else {
        None
    };

    let copied_count = if source.is_file() {
        let dest_path = if dest.is_dir() {
            dest.join(source.file_name().unwrap())
        } else {
            dest.to_path_buf()
        };

        copy_and_hash_file(source, &dest_path, &args, config, &mut db_conn).await?;
        1
    } else {
        let base_source = source.canonicalize()?;
        let base_dest = dest.canonicalize().unwrap_or_else(|_| dest.to_path_buf());
        copy_directory(&base_source, &base_dest, &args, config, &mut db_conn).await?
    };

    info!("Successfully copied {} files", copied_count);
    Ok(())
}
