use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::Path;
use sqlx::{query_builder::QueryBuilder, Connection, SqliteConnection};
use log::{info, warn};
use serde_json::{Map, Value};

use crate::configuration::{Args, Config};
use crate::utils::Error;
use crate::walkthedir;
use hasher::{Hasher, HashConfig};

async fn insert_hashes_sql(
    config: &Config,
    file_path: &Path,
    size: usize,
    hashes: Vec<(&str, Vec<u8>)>,
    db_conn: &mut SqliteConnection,
) -> Result<(), Error> {
    let mut query_builder: QueryBuilder<sqlx::Sqlite> = QueryBuilder::new("INSERT INTO ");
    query_builder.push(config.database.table_name.clone());

    let mut sep = query_builder.separated(", ");
    sep.push_unseparated(" (");
    sep.push("file_path");
    sep.push("file_size");
    for (hash_name, _) in &hashes {
        sep.push(*hash_name);
    }

    let mut sep = query_builder.separated(", ");
    sep.push_unseparated(") VALUES (");
    sep.push_bind(file_path.display().to_string());
    sep.push_bind(size as f64);
    for (_, hash_data) in &hashes {
        sep.push_bind(hash_data.as_slice());
    }
    sep.push_unseparated(");");

    let query = query_builder.build();
    query.execute(db_conn).await?;

    Ok(())
}

fn write_hashes_json(
    args: &Args,
    file_path: &Path,
    file_size: usize,
    hashes: Vec<(&str, Vec<u8>)>,
) -> Result<(), Error> {
    let mut map = Map::new();
    create_dir_all(&args.json_output_path)?;

    map.insert("file_path".to_string(), Value::from(file_path.display().to_string()));
    map.insert("file_size".to_string(), Value::from(file_size));

    for (hash_name, hash_data) in hashes {
        map.insert(hash_name.to_string(), Value::from(hex::encode(hash_data)));
    }

    let json_obj = Value::Object(map).to_string();

    // Use SHA-256 of the JSON as filename
    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(json_obj.as_bytes());
    let filename = hex::encode(hasher.finalize());

    let output_path = format!("{}/{}.json", args.json_output_path, filename);
    info!("Writing output hash file to {}", output_path);

    let mut output_file = File::create(output_path)?;
    write!(output_file, "{}\n", json_obj)?;

    Ok(())
}

fn create_hash_config(config: &Config) -> HashConfig {
    HashConfig {
        crc32: config.hashes.crc32.unwrap_or(false),
        md2: config.hashes.md2.unwrap_or(false),
        md4: config.hashes.md4.unwrap_or(false),
        md5: config.hashes.md5.unwrap_or(false),
        sha1: config.hashes.sha1.unwrap_or(false),
        sha224: config.hashes.sha224.unwrap_or(false),
        sha256: config.hashes.sha256.unwrap_or(false),
        sha384: config.hashes.sha384.unwrap_or(false),
        sha512: config.hashes.sha512.unwrap_or(false),
        sha3_224: config.hashes.sha3_224.unwrap_or(false),
        sha3_256: config.hashes.sha3_256.unwrap_or(false),
        sha3_384: config.hashes.sha3_384.unwrap_or(false),
        sha3_512: config.hashes.sha3_512.unwrap_or(false),
        keccak224: config.hashes.keccak224.unwrap_or(false),
        keccak256: config.hashes.keccak256.unwrap_or(false),
        keccak384: config.hashes.keccak384.unwrap_or(false),
        keccak512: config.hashes.keccak512.unwrap_or(false),
        blake2s256: config.hashes.blake2s256.unwrap_or(false),
        blake2b512: config.hashes.blake2b512.unwrap_or(false),
        belt_hash: config.hashes.belt_hash.unwrap_or(false),
        whirlpool: config.hashes.whirlpool.unwrap_or(false),
        tiger: config.hashes.tiger.unwrap_or(false),
        tiger2: config.hashes.tiger2.unwrap_or(false),
        streebog256: config.hashes.streebog256.unwrap_or(false),
        streebog512: config.hashes.streebog512.unwrap_or(false),
        ripemd128: config.hashes.ripemd128.unwrap_or(false),
        ripemd160: config.hashes.ripemd160.unwrap_or(false),
        ripemd256: config.hashes.ripemd256.unwrap_or(false),
        ripemd320: config.hashes.ripemd320.unwrap_or(false),
        fsb160: config.hashes.fsb160.unwrap_or(false),
        fsb224: config.hashes.fsb224.unwrap_or(false),
        fsb256: config.hashes.fsb256.unwrap_or(false),
        fsb384: config.hashes.fsb384.unwrap_or(false),
        fsb512: config.hashes.fsb512.unwrap_or(false),
        sm3: config.hashes.sm3.unwrap_or(false),
        gost94_cryptopro: config.hashes.gost94_cryptopro.unwrap_or(false),
        gost94_test: config.hashes.gost94_test.unwrap_or(false),
        gost94_ua: config.hashes.gost94_ua.unwrap_or(false),
        gost94_s2015: config.hashes.gost94_s2015.unwrap_or(false),
        groestl224: config.hashes.groestl224.unwrap_or(false),
        groestl256: config.hashes.groestl256.unwrap_or(false),
        groestl384: config.hashes.groestl384.unwrap_or(false),
        groestl512: config.hashes.groestl512.unwrap_or(false),
        shabal192: config.hashes.shabal192.unwrap_or(false),
        shabal224: config.hashes.shabal224.unwrap_or(false),
        shabal256: config.hashes.shabal256.unwrap_or(false),
        shabal384: config.hashes.shabal384.unwrap_or(false),
        shabal512: config.hashes.shabal512.unwrap_or(false),
    }
}

pub async fn process_file(
    file_path: &Path,
    config: &Config,
    args: &Args,
    db_conn: &mut Option<SqliteConnection>
) -> Result<(), Error> {
    let mut hasher = Hasher::new(create_hash_config(config));

    match hasher.hash_file(file_path) {
        Ok((file_size, hashes)) => {
            info!("Successfully hashed {}", file_path.display());
            for (name, hash) in &hashes {
                info!("{}: {}", name, hex::encode(hash));
            }

            if args.dry_run {
                return Ok(());
            }

            if let Some(conn) = db_conn {
                insert_hashes_sql(config, file_path, file_size, hashes.clone(), conn).await?;
            }

            if args.json_out {
                write_hashes_json(args, file_path, file_size, hashes)?;
            }

            Ok(())
        }
        Err(e) => {
            warn!("Failed to hash {}: {:?}", file_path.display(), e);
            Err(e.into())
        }
    }
}

pub async fn process_stdin(
    config: &Config,
    file_path: &str,
    conn: &mut SqliteConnection
) -> Result<(), Error> {
    let mut buffer = Vec::new();
    std::io::stdin().read_to_end(&mut buffer)?;

    let mut hasher = Hasher::new(create_hash_config(config));
    let hashes = hasher.hash_single_buffer(&buffer)?;

    insert_hashes_sql(
        config,
        Path::new(file_path),
        buffer.len(),
        hashes,
        conn,
    ).await?;

    Ok(())
}

pub async fn process_directory(
    path_to_hash: &Path,
    args: &Args,
    config: &Config
) -> Result<(), Error> {
    let mut file_count: usize = 0;
    let mut db_conn = if args.sql_out {
        Some(SqliteConnection::connect(&config.database.db_string).await?)
    } else {
        None
    };

    for entry in walkthedir!(path_to_hash, args) {
        if let Ok(entry) = entry {
            if !entry.path().is_dir() {
                file_count += 1;
                if file_count <= args.skip_files {
                    info!(
                        "Skipping ({}/{}) file {}",
                        file_count,
                        args.skip_files,
                        entry.path().display()
                    );
                    continue;
                }

                process_file(entry.path(), config, args, &mut db_conn).await?;
            }
        } else {
            warn!("Unexpected error accessing a walkdir entry");
        }
    }

    info!(
        "Successfully processed {} files at path: {}",
        file_count,
        path_to_hash.display()
    );

    Ok(())
}
