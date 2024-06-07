use std::cmp::min;
use std::fs::{create_dir_all, File};
use std::io::{BufReader, Read, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::time::Instant;

use crc32fast;
use digest::DynDigest;
use hex;
use log::{info, warn};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use sqlx::{query_builder::QueryBuilder, Connection};
use sqlx::{Sqlite, SqliteConnection};
use walkdir::WalkDir;

use crate::configuration::{get_hashes, Args, Config, Hashes};
use crate::utils::Error;
use crate::{arclock, startthread, walkthedir};

// Read 1 GiB of the file at a time when dealing with large files
const CHUNK_SIZE: usize = 512 * 1024 * 1024;

// At a certain file size the overhead to start/stop threads reduces performance compared to sequential.
// By setting this there can be some minor performance gains with small files.
// Must be smaller than CHUNK_SIZE, value is currently a complete guess.
const SEQUENTIAL_SIZE: usize = 32 * 1024 * 1024;

fn open_file<'a>(file_path: &'a Path) -> Result<(BufReader<File>, usize), Error> {
    let input_file = File::open(file_path)?;
    let file_size = input_file.metadata()?.len() as usize;
    let file_reader = BufReader::new(input_file);

    Ok((file_reader, file_size))
}

fn make_buffer(file_size: usize) -> (Arc<RwLock<Vec<u8>>>, usize) {
    let buffer_size = min(file_size, CHUNK_SIZE);
    let buffer = Arc::new(RwLock::new(vec![0; buffer_size]));

    (buffer, buffer_size)
}

fn hash_buffer_sequential(
    buffer: &Arc<RwLock<Vec<u8>>>,
    hashes: &mut MutexGuard<Vec<(&str, Arc<Mutex<dyn DynDigest + Send>>)>>,
    hash_crc32: bool,
    crc32_hasher: &Arc<Mutex<crc32fast::Hasher>>,
) -> Result<(), Error> {
    let buffer_clone = buffer.clone();

    // Calculate each hash sequentially
    for (_hash_name, hash_mutex) in hashes.iter_mut() {
        hash_mutex.lock()?.update(buffer_clone.read()?.as_slice());
    }

    // Hash CRC32 (if applicable)
    if hash_crc32 {
        crc32_hasher.lock()?.update(buffer_clone.read()?.as_slice());
    }

    Ok(())
}

fn hash_buffer_threaded<'a>(
    buffer: &Arc<RwLock<Vec<u8>>>,
    hashes: &mut MutexGuard<Vec<(&str, Arc<Mutex<dyn DynDigest + Send>>)>>,
    hash_crc32: bool,
    crc32_hasher: &Arc<Mutex<crc32fast::Hasher>>,
) -> Result<Vec<JoinHandle<Result<(), Error>>>, Error> {
    let mut threads: Vec<JoinHandle<Result<(), Error>>> = Vec::new();

    for (_hash_name, hash_mutex) in hashes.iter_mut() {
        startthread!(threads, buffer, hash_mutex);
    }

    if hash_crc32 {
        startthread!(threads, buffer, crc32_hasher);
    }

    Ok(threads)
}

pub async fn hash_stdin<'a>(config: &Config, file_path: &'a str) -> Result<(), Error> {
    info!("Hashing from stdin.");
    let start_time = Instant::now();

    let hashes_arc = get_hashes(&config.hashes);
    let mut hashes = arclock!(hashes_arc);
    let crc32_hasher = Arc::new(Mutex::new(crc32fast::Hasher::new()));

    let mut db_conn = SqliteConnection::connect(&config.database.db_string)
        .await
        .expect("Failed to connect to db!");

    let mut raw_buffer: Vec<u8> = Vec::new();
    std::io::stdin().read_to_end(&mut raw_buffer)?;

    let file_size = raw_buffer.len();
    let buffer = Arc::new(RwLock::new(raw_buffer));

    if file_size < SEQUENTIAL_SIZE {
        hash_buffer_sequential(&buffer, &mut hashes, config.hashes.crc32.unwrap(), &crc32_hasher)?;
    } else {
        let threads = hash_buffer_threaded(&buffer, &mut hashes, config.hashes.crc32.unwrap(), &crc32_hasher)?;

        // Wait for all threads to finish processing
        for handle in threads.into_iter() {
            handle.join().unwrap()?;
        }
    }

    let mut final_hashes: Vec<(&str, Vec<u8>)> = Vec::new();

    info!("Successfully hashed file from stdin in {:.2?}", start_time.elapsed());
    info!("File name (in output): {}", file_path);
    info!("File size: {} bytes", file_size);

    // Get the result of each hash in a consistent format
    // CRC32 is special
    if config.hashes.crc32.unwrap() {
        final_hashes.push((
            "crc32",
            arclock!(crc32_hasher)
                .clone()
                .finalize()
                .to_be_bytes()
                .to_vec(),
        ));
        info!("crc32: {}", hex::encode(&final_hashes[0].1));
    }

    for (hash_name, hash_mutex) in hashes.drain(..) {
        let hash_vec = arclock!(hash_mutex).finalize_reset().to_vec();

        info!("{}: {}", hash_name, hex::encode(&hash_vec));

        final_hashes.push((hash_name, hash_vec));
    }

    insert_hashes_sql(
        config,
        Path::new(file_path),
        &(file_size, final_hashes),
        &mut db_conn,
    )
    .await
    .expect("Failed to insert hashes! This is likely a schema error.");

    Ok(())
}

pub fn hash_file<'a>(
    file_path: &'a Path,
    config: &Hashes,
) -> Result<(usize, Vec<(&'a str, Vec<u8>)>), Error> {
    info!("Hashing file: {}", file_path.display());
    let start_time = Instant::now();

    let (mut file_reader, file_size) = open_file(file_path)?;

    let hashes_arc = get_hashes(config);
    let mut hashes = arclock!(hashes_arc);
    let crc32_hasher = Arc::new(Mutex::new(crc32fast::Hasher::new()));

    let (mut buffer, mut buffer_size) = make_buffer(file_size);
    let mut bytes_read = file_reader.read(buffer.write()?.as_mut())?;

    loop {
        if bytes_read == 0 {
            break;
        }

        if bytes_read < SEQUENTIAL_SIZE {
            hash_buffer_sequential(&buffer, &mut hashes, config.crc32.unwrap(), &crc32_hasher)?;

            bytes_read = file_reader.read(buffer.write()?.as_mut())?;
        } else {
            // Ensure only the amount of data that was read will be hashed
            if bytes_read < buffer_size {
                buffer.write()?.resize(bytes_read, 0);
            }

            let threads = hash_buffer_threaded(&buffer, &mut hashes, config.crc32.unwrap(), &crc32_hasher)?;

            // Read the next buffer while the hashing threads are running
            let (buffer2, buffer2_size) = make_buffer(file_size);
            bytes_read = file_reader.read(buffer2.write()?.as_mut())?;

            // Wait for all threads to finish processing
            for handle in threads.into_iter() {
                handle.join().unwrap()?;
            }

            drop(buffer.write()?);
            buffer = buffer2;
            buffer_size = buffer2_size;
        }
    }

    let mut final_hashes: Vec<(&str, Vec<u8>)> = Vec::new();

    info!("Successfully hashed file in {:.2?}", start_time.elapsed());
    info!("File name: {}", file_path.display());
    info!("File size: {} bytes", file_size);

    // Get the result of each hash in a consistent format
    if config.crc32.unwrap() {
        final_hashes.push((
            "crc32",
            arclock!(crc32_hasher)
                .clone()
                .finalize()
                .to_be_bytes()
                .to_vec(),
        ));
        info!("crc32: {}", hex::encode(&final_hashes[0].1));
    }

    for (hash_name, hash_mutex) in hashes.drain(..) {
        let hash_vec = arclock!(hash_mutex).finalize_reset().to_vec();

        info!("{}: {}", hash_name, hex::encode(&hash_vec));

        final_hashes.push((hash_name, hash_vec));
    }

    Ok((file_size, final_hashes))
}

fn write_hashes_json(
    config: &Args,
    file_path: &Path,
    file_size: usize,
    hashes: Vec<(&str, Vec<u8>)>,
) -> Result<(), Error> {
    let mut map = Map::new();

    create_dir_all(config.json_output_path.clone())?;

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
    Digest::update(&mut sha256_hasher, json_obj.as_bytes());
    let sha256_hash: String = hex::encode(sha256_hasher.finalize());

    let output_path = format!("{}/{}.json", config.json_output_path, sha256_hash);
    info!("Writing output hash file to {}", output_path);

    let mut output_file = File::create(output_path)?;
    write!(output_file, "{}\n", json_obj)?;

    Ok(())
}

async fn insert_hashes_sql(
    config: &Config,
    file_path: &Path,
    hashes: &(usize, Vec<(&str, Vec<u8>)>),
    db_conn: &mut SqliteConnection,
) -> Result<(), Error> {
    let mut query_builder: QueryBuilder<Sqlite> = QueryBuilder::new("INSERT INTO ");

    query_builder.push(config.database.table_name.to_string());

    let mut sep = query_builder.separated(", ");
    sep.push_unseparated(" (");
    sep.push("file_path");
    sep.push("file_size");
    for hash in &hashes.1 {
        sep.push(hash.0.to_string());
    }

    let mut sep = query_builder.separated(", ");
    sep.push_unseparated(") VALUES (");
    sep.push_bind(file_path.display().to_string());
    sep.push_bind(hashes.0 as f64);
    for hash in &hashes.1 {
        sep.push_bind(hash.1.as_slice());
    }

    sep.push_unseparated(");");

    let query = query_builder.build();
    query.execute(db_conn).await?;

    Ok(())
}

// Uses hash_file_threaded on every file in a directory up to the given depth
pub async fn hash_dir(path_to_hash: &Path, args: &Args, config: &Config) -> Result<(), Error> {
    let mut file_count: usize = 0;

    info!(
        "Hashing path: {} up to {} level(s) of depth.",
        path_to_hash.display(),
        args.max_depth
    );

    let mut db_conn: Option<SqliteConnection> = None;
    if args.sql_out {
        db_conn = Some(SqliteConnection::connect(&config.database.db_string)
            .await
            .expect("Failed to open sqlite database!"));
    }

    for entry in walkthedir!(path_to_hash, args) {
        if let Ok(entry_ok) = entry {
            // Only hash files, not directories
            if !entry_ok.path().is_dir() {
                // Skipping a given number of files in the args (for interruption recovery)
                file_count += 1;
                if file_count <= args.skip_files {
                    info!(
                        "Skipping ({}/{}) file {}",
                        file_count,
                        args.skip_files,
                        entry_ok.path().display()
                    );
                    continue;
                }

                match hash_file(entry_ok.path(), &config.hashes) {
                    Ok(hashes) => {
                        if args.dry_run {
                            continue;
                        }

                        if args.sql_out {
                            if let Some(conn) = &mut db_conn {
                                insert_hashes_sql(config, entry_ok.path(), &hashes, conn)
                                    .await
                                    .expect("Failed to insert hashes! This is likely a schema error.");
                            }
                        }

                        if args.json_out {
                            write_hashes_json(args, entry_ok.path(), hashes.0, hashes.1)?;
                        }
                    }
                    Err(err) => {
                        warn!("Failed to access file at {}, skipping. {:?}", entry_ok.path().display(), err);
                    }
                }
            }
        } else {
            warn!("Unexpected error accessing a walkdir entry (this shouldn't happen)!");
        }
    }

    info!(
        "Successfully hashed {} files at path: {}",
        file_count,
        path_to_hash.display()
    );

    Ok(())
}
