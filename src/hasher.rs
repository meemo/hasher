use std::cmp::min;
use std::fs::{create_dir_all, File};
use std::io::{self, BufReader, Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;

use crc32fast;
use hex;
use log::{info, warn};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::configuration::{get_hashes, HasherArgs, HasherHashes};

/**
 * hasher.rs
 *
 * Core functions for hashing.
 */

// Read 1 GiB of the file at a time when dealing with large files
const CHUNK_SIZE: usize = 1024 * 1024 * 1024;

#[derive(Debug)]
pub enum Error {
    IO,
    Poison,
}

impl From<std::io::Error> for Error {
    fn from(_value: std::io::Error) -> Self {
        Error::IO
    }
}

impl<T> From<std::sync::PoisonError<T>> for Error {
    fn from(_value: std::sync::PoisonError<T>) -> Self {
        Error::Poison
    }
}

impl From<walkdir::Error> for Error {
    fn from(_value: walkdir::Error) -> Self {
        Error::IO
    }
}

// Where the magic happens
pub fn hash_file_threaded<'a>(
    file_path: &'a Path,
    config: &HasherHashes,
) -> Result<(usize, Vec<(&'a str, Vec<u8>)>), Error> {
    info!("Beginning to hash file: {}", file_path.display());

    let start_time = Instant::now();

    let input_file = File::open(file_path)?;
    let file_size: usize = input_file.metadata()?.len() as usize;
    let mut file_reader = BufReader::new(input_file);

    let buffer_size: usize = min(file_size, CHUNK_SIZE);
    let buffer = Arc::new(RwLock::new(vec![0; buffer_size]));

    let hashes_arc = get_hashes(config);
    let mut hashes = hashes_arc.lock().unwrap();

    let crc32_hasher = Arc::new(Mutex::new(crc32fast::Hasher::new()));

    // Hash the entire buffer
    loop {
        let mut threads: Vec<JoinHandle<Result<(), Error>>> = Vec::new();

        let bytes_read = file_reader.read(buffer.write()?.as_mut())?;

        if bytes_read == 0 {
            break;
        }

        // Ensure only the amount of data that was read will be hashed
        if bytes_read < buffer_size {
            buffer.write()?.resize(bytes_read, 0);
        }

        // Start threads for each hash
        for (_hash_name, hash_mutex) in hashes.iter_mut() {
            let buffer_clone = buffer.clone();
            let hash_clone = hash_mutex.clone();

            threads.push(thread::spawn(move || {
                hash_clone.lock()?.update(buffer_clone.read()?.as_slice());
                Ok(())
            }));
        }

        // Start thread for CRC32 (if applicable)
        if config.crc32 {
            let buffer_clone = buffer.clone();
            let hash_clone = crc32_hasher.clone();

            threads.push(thread::spawn(move || {
                hash_clone.lock()?.update(buffer_clone.read()?.as_slice());
                Ok(())
            }));
        }

        // Wait for all threads to finish processing
        for handle in threads.into_iter() {
            handle.join().unwrap()?;
        }
    }

    let mut final_hashes: Vec<(&str, Vec<u8>)> = Vec::new();

    // Get the result of each hash in a consistent format
    if config.crc32 {
        final_hashes.push((
            "crc32",
            crc32_hasher
                .lock()
                .unwrap()
                .clone()
                .finalize()
                .to_be_bytes()
                .to_vec(),
        ));
    }

    info!("Successfully hashed file in {:.2?}", start_time.elapsed());
    info!("File name: {}", file_path.display());
    info!("File size: {} bytes", file_size);
    info!("crc32: {}", hex::encode(&final_hashes[0].1));

    for (hash_name, hash_mutex) in hashes.drain(..) {
        let hash_vec = hash_mutex.lock().unwrap().finalize_reset().to_vec();
        info!("{}: {}", hash_name, hex::encode(&hash_vec));

        final_hashes.push((hash_name, hash_vec));
    }

    Ok((file_size, final_hashes))
}

fn write_hashes_json(
    config: &HasherArgs,
    file_path: &Path,
    file_size: usize,
    hashes: Vec<(&str, Vec<u8>)>,
) {
    let mut map = Map::new();

    create_dir_all(config.json_output_path.clone()).expect("Failed to create output directory.");

    map.insert(
        "file_path".to_string(),
        Value::from(file_path.display().to_string()),
    );
    map.insert("file_size".to_string(), Value::from(file_size));

    for (hash_name, hash_data) in hashes.iter() {
        map.insert(hash_name.to_string(), Value::from(hex::encode(hash_data)));
    }

    let json_obj = Value::Object(map).to_string();

    let mut sha256_hasher = Sha256::new();
    sha256_hasher.update(json_obj.as_bytes());
    let sha256_hash: String = hex::encode(sha256_hasher.finalize());

    let output_path = format!("{}/{}.json", config.json_output_path, sha256_hash);
    info!("Writing output hash file to {}", output_path);
    let mut output_file = File::create(output_path).expect("Failed to open file!");

    write!(output_file, "{}\n", json_obj).expect("Failed to write to file.");
}

// Uses hash_file_threaded on every file in a directory up to the given depth
pub fn hash_dir(
    path_to_hash: &Path,
    config: &HasherArgs,
    config_hashes: &HasherHashes,
) -> Result<(), Error> {
    info!(
        "Hashing path: {} up to {} level(s) of depth.",
        path_to_hash.display(),
        config.max_depth
    );

    for entry in WalkDir::new(path_to_hash)
        .min_depth(0)
        .max_depth(config.max_depth)
        .follow_links(!config.no_follow_symlinks)
        .contents_first(!config.breadth_first)
        .sort_by_file_name()
    {
        match entry {
            Ok(entry_ok) => {
                if !entry_ok.path().is_dir() {
                    if let Ok(hashes) = hash_file_threaded(entry_ok.path(), config_hashes) {
                        write_hashes_json(config, entry_ok.path(), hashes.0, hashes.1);
                    } else {
                        warn!(
                            "Failed to hash file at path {}, skipping",
                            entry_ok.path().display()
                        );
                    }
                }
            }
            Err(err) => {
                let path = err.path().unwrap_or(Path::new("")).display();
                warn!("Failed to access entry at {}", path);

                if let Some(inner) = err.io_error() {
                    match inner.kind() {
                        io::ErrorKind::PermissionDenied => {
                            warn!("Missing permission to read entry: {}, skipping", inner);
                        }
                        _ => {
                            warn!("Unexpected error at entry: {}, skipping", inner);
                        }
                    }
                }
            }
        }
    }

    info!("Successfully hashed path: {}", path_to_hash.display());

    Ok(())
}
