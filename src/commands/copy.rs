use std::fs::File;
use std::io::{BufReader, BufWriter, Read};
use std::path::{Path, PathBuf};

use log::{debug, error, info};
use serde_json::json;
use sqlx::Connection;
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

fn output_skip_json(path: &Path, reason: &str, pretty: bool) {
    let mut skip_map = serde_json::Map::new();
    skip_map.insert("status".to_string(), json!("skipped"));
    skip_map.insert("file_path".to_string(), json!(path.display().to_string()));
    skip_map.insert("reason".to_string(), json!(reason));

    let output = if pretty {
        serde_json::to_string_pretty(&skip_map)
    } else {
        serde_json::to_string(&skip_map)
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

fn get_file_data(path: &Path) -> Result<(bool, Vec<u8>), Error> {
    // Get initial metadata for size check and later comparison
    let initial_metadata = std::fs::metadata(path)?;

    let mut file = BufReader::new(File::open(path)?);
    let mut data = Vec::new();
    file.read_to_end(&mut data)?;

    // Check if file changed during read
    let final_metadata = std::fs::metadata(path)?;
    if initial_metadata.modified()? != final_metadata.modified()?
        || initial_metadata.len() != final_metadata.len()
    {
        return Err(Error::FileChanged);
    }

    let compressor = compression::get_compressor(compression::CompressionType::Gzip, 6);
    let is_compressed = compressor.is_compressed_path(path);

    if is_compressed {
        // Verify it's actually a gzip file by trying to decompress
        match compression::decompress_bytes(&data, compression::CompressionType::Gzip) {
            Ok(decompressed) => Ok((true, decompressed)),
            Err(_) => Err(Error::Config("Invalid gzip file".to_string())),
        }
    } else {
        Ok((false, data))
    }
}

fn file_existing(
    source: &Path,
    dest: &Path,
    args: &HasherCopyArgs,
    _config: &Config,
) -> Result<bool, Error> {
    if !args.skip_existing {
        return Ok(false);
    }
    
    if !dest.exists() {
        return Ok(false);
    }

    // Get initial metadata to detect changes during comparison
    let initial_source_meta = std::fs::metadata(source)?;
    let initial_dest_meta = std::fs::metadata(dest)?;

    // Handle symlinks consistently
    if initial_source_meta.file_type().is_symlink() != initial_dest_meta.file_type().is_symlink() {
        debug!("Symlink status mismatch between source and destination");
        return Ok(false);
    }

    let compressor = compression::get_compressor(
        compression::CompressionType::Gzip,
        args.hash_options.compression_level,
    );
    let source_compressed = compressor.is_compressed_path(source);
    let dest_compressed = compressor.is_compressed_path(dest);

    // Ensure source file exists before trying to read it
    if !source.exists() {
        return Err(Error::Config(format!("Source file does not exist: {}", source.display())));
    }

    let source_data = if (source_compressed && args.hash_options.decompress) || 
                     (source_compressed && args.hash_options.hash_uncompressed) {
        let compressed = std::fs::read(source)?;
        compression::decompress_bytes(&compressed, compression::CompressionType::Gzip)?
    } else if !source_compressed && args.hash_options.hash_compressed {
        let data = std::fs::read(source)?;
        compression::compress_bytes(
            &data,
            compression::CompressionType::Gzip,
            args.hash_options.compression_level,
        )
        .map_err(Error::from)?
    } else {
        std::fs::read(source)?
    };

    // Double-check destination exists before trying to read it
    if !dest.exists() {
        debug!("Destination file disappeared after initial check: {}", dest.display());
        return Ok(false);
    }

    let dest_data = if (dest_compressed && args.hash_options.decompress) ||
                     (dest_compressed && args.hash_options.hash_uncompressed) {
        let compressed = std::fs::read(dest)?;
        compression::decompress_bytes(&compressed, compression::CompressionType::Gzip)?
    } else if !dest_compressed && args.hash_options.hash_compressed {
        let data = std::fs::read(dest)?;
        compression::compress_bytes(
            &data,
            compression::CompressionType::Gzip,
            args.hash_options.compression_level,
        )
        .map_err(Error::from)?
    } else {
        std::fs::read(dest)?
    };

    // Verify files haven't changed during comparison
    let final_source_meta = std::fs::metadata(source)?;
    let final_dest_meta = std::fs::metadata(dest)?;

    if initial_source_meta.modified()? != final_source_meta.modified()?
        || initial_source_meta.len() != final_source_meta.len()
        || initial_dest_meta.modified()? != final_dest_meta.modified()?
        || initial_dest_meta.len() != final_dest_meta.len()
    {
        return Err(Error::FileChanged);
    }

    // If sizes don't match, we should copy
    if source_data.len() != dest_data.len() {
        debug!(
            "Size mismatch: source={}, dest={}",
            source_data.len(),
            dest_data.len()
        );
        return Ok(false);
    }

    // If hash comparison is disabled, we can skip at this point
    if args.no_hash_existing {
        if !args.hash_options.silent_failures {
            output_skip_json(dest, "size match", args.hash_options.pretty_json);
        } else {
            info!("Skipping existing file (size match): {}", dest.display());
        }
        return Ok(true);
    }

    // Compare hashes using SHA256
    let mut source_hasher = Hasher::new(HashConfig {
        sha256: true,
        ..Default::default()
    });
    let mut dest_hasher = Hasher::new(HashConfig {
        sha256: true,
        ..Default::default()
    });

    let source_hashes = source_hasher.hash_single_buffer(&source_data)?;
    let dest_hashes = dest_hasher.hash_single_buffer(&dest_data)?;

    let source_sha256 = source_hashes
        .iter()
        .find(|(name, _)| *name == "sha256")
        .map(|(_, hash)| hash);
    let dest_sha256 = dest_hashes
        .iter()
        .find(|(name, _)| *name == "sha256")
        .map(|(_, hash)| hash);

    if let (Some(source_hash), Some(dest_hash)) = (source_sha256, dest_sha256) {
        if source_hash == dest_hash {
            if !args.hash_options.silent_failures {
                output_skip_json(dest, "hash match", args.hash_options.pretty_json);
            } else {
                info!("Skipping existing file (hash match): {}", dest.display());
            }
            return Ok(true);
        }
        debug!("Hash mismatch between source and destination");
    }

    Ok(false)
}

async fn _hash_file(
    path: &Path,
    hasher: &mut Hasher,
    args: &HasherCopyArgs,
    config: &Config,
    db_conn: &mut Option<sqlx::SqliteConnection>,
) -> Result<(), Error> {
    match hasher.hash_file(path) {
        Ok((file_size, hashes)) => {
            process_hash_results(path, file_size, &hashes, args, config, db_conn).await?;
            Ok(())
        }
        Err(e) => {
            if !args.hash_options.fail_fast {
                error!("Failed to hash {}: {}", path.display(), e);
                Ok(())
            } else {
                Err(Error::from(e))
            }
        }
    }
}

async fn _hash_compressed_file(
    source: &Path,
    hasher: &mut Hasher,
    args: &HasherCopyArgs,
    config: &Config,
    db_conn: &mut Option<sqlx::SqliteConnection>,
) -> Result<(), Error> {
    // Hash compressed state
    _hash_file(source, hasher, args, config, db_conn).await?;

    // Hash decompressed state
    if let Ok((_, data)) = get_file_data(source) {
        match hasher.hash_single_buffer(&data) {
            Ok(hashes) => {
                process_hash_results(source, data.len(), &hashes, args, config, db_conn).await?;
            }
            Err(e) => {
                if !args.hash_options.fail_fast {
                    error!(
                        "Failed to hash decompressed data for {}: {}",
                        source.display(),
                        e
                    );
                } else {
                    return Err(Error::from(e));
                }
            }
        }
    }
    Ok(())
}

fn get_final_dest(dest: &Path, args: &HasherCopyArgs) -> PathBuf {
    let compressor = compression::get_compressor(
        compression::CompressionType::Gzip,
        args.hash_options.compression_level,
    );
    
    if args.hash_options.compress {
        // Don't append .gz if the file is already compressed
        if !compressor.is_compressed_path(dest) {
            return dest.with_extension(format!(
                "{}{}",
                dest.extension().unwrap_or_default().to_string_lossy(),
                compressor.extension()
            ));
        }
    } else if args.hash_options.decompress {
        // Remove .gz extension if file is compressed
        if compressor.is_compressed_path(dest) {
            // Get the extension without the .gz
            if let Some(ext) = dest.extension() {
                let ext_str = ext.to_string_lossy();
                if ext_str == "gz" {
                    // No extension before .gz
                    return dest.with_extension("");
                } else if ext_str.ends_with(".gz") {
                    // Has extension before .gz (e.g., .tar.gz)
                    let base_ext = ext_str.trim_end_matches(".gz");
                    if !base_ext.is_empty() {
                        return dest.with_extension(base_ext);
                    }
                }
            }
        }
    }
    dest.to_path_buf()
}

fn copy_file(source: &Path, dest: &Path, args: &HasherCopyArgs) -> Result<(), Error> {
    let compressor = compression::get_compressor(
        compression::CompressionType::Gzip,
        args.hash_options.compression_level,
    );
    let source_compressed = compressor.is_compressed_path(source);

    if args.hash_options.compress && !source_compressed {
        // Compress uncompressed source
        let source_data = std::fs::read(source)?;
        let compressed = compression::compress_bytes(
            &source_data,
            compression::CompressionType::Gzip,
            args.hash_options.compression_level,
        )
        .map_err(Error::from)?;
        std::fs::write(dest, compressed)?;
    } else if args.hash_options.decompress && source_compressed {
        // Decompress compressed source
        let compressed = std::fs::read(source)?;
        let decompressed =
            compression::decompress_bytes(&compressed, compression::CompressionType::Gzip)?;
        std::fs::write(dest, decompressed)?;
    } else {
        // Direct copy
        let mut reader = BufReader::new(File::open(source)?);
        let mut writer = BufWriter::new(File::create(dest)?);
        std::io::copy(&mut reader, &mut writer)?;
    }
    Ok(())
}

async fn hash_file_based_on_options(
    source: &Path,
    final_dest: &Path,
    args: &HasherCopyArgs,
    config: &Config,
    db_conn: &mut Option<sqlx::SqliteConnection>,
) -> Result<(), Error> {
    let mut hasher = Hasher::new(HashConfig::from(&config.hashes));
    let compressor = compression::get_compressor(compression::CompressionType::Gzip, 6);
    let is_compressed = compressor.is_compressed_path(source);

    if args.hash_options.hash_both {
        if is_compressed {
            _hash_compressed_file(source, &mut hasher, args, config, db_conn).await?;
        } else {
            let path_to_hash = if args.store_source_path {
                source
            } else {
                final_dest
            };
            _hash_file(path_to_hash, &mut hasher, args, config, db_conn).await?;
        }
    } else if args.hash_options.hash_uncompressed && is_compressed {
        // Handle compressed source when hash_uncompressed is set
        // Get the file data and decompress it
        if let Ok((_, data)) = get_file_data(source) {
            match hasher.hash_single_buffer(&data) {
                Ok(hashes) => {
                    let path_to_store = if args.store_source_path {
                        source
                    } else {
                        final_dest
                    };
                    process_hash_results(path_to_store, data.len(), &hashes, args, config, db_conn).await?;
                }
                Err(e) => {
                    if !args.hash_options.fail_fast {
                        error!(
                            "Failed to hash decompressed data for {}: {}",
                            source.display(),
                            e
                        );
                    } else {
                        return Err(Error::from(e));
                    }
                }
            }
        }
    } else {
        let path_to_hash = if args.store_source_path {
            source
        } else if args.hash_options.hash_compressed {
            final_dest
        } else {
            source
        };
        _hash_file(path_to_hash, &mut hasher, args, config, db_conn).await?;
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
    if args.hash_options.dry_run {
        return Ok(());
    }

    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let final_dest = get_final_dest(dest, args);

    if file_existing(source, &final_dest, args, config)? {
        return Ok(());
    }

    copy_file(source, &final_dest, args)?;
    hash_file_based_on_options(source, &final_dest, args, config, db_conn).await?;

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
                .map_err(|_| Error::Config("Failed to strip prefix".to_string()))?;
            let dest_path = base_dest.join(rel_path);

            if let Err(e) = copy_and_hash_file(path, &dest_path, args, config, db_conn).await {
                let err_msg = format!("Failed to copy {}: {}", path.display(), e);
                if !args.hash_options.fail_fast {
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
        return Err(Error::Config("Source path does not exist".to_string()));
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
        // Get absolute paths without the \\?\ prefix on Windows
        let base_source = if source.is_absolute() {
            source.to_path_buf()
        } else {
            std::env::current_dir()?.join(source)
        };
        let base_dest = if dest.is_absolute() {
            dest.to_path_buf()
        } else {
            std::env::current_dir()?.join(dest)
        };
        copy_directory(&base_source, &base_dest, &args, config, &mut db_conn).await?
    };

    info!("Successfully copied {} files", copied_count);
    Ok(())
}
