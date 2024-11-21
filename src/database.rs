use std::path::{PathBuf, Path};
use std::time::Duration;

use tokio::time::sleep;
use sqlx::{query_builder::QueryBuilder, Row, Column};
use sqlx::{SqliteConnection, Connection, sqlite::SqliteConnectOptions};
use log::info;

use crate::utils::Error;
use crate::configuration::Config;

const DB_RETRY_DELAY: Duration = Duration::from_millis(100);
const MAX_DB_RETRIES: u32 = 3;

// The first chunk of the database is down below in the code to prevent injection possibilities
const HASHES: &str = "(
    file_path text not null,
    file_size numeric not null,
    crc32 blob,
    md2 blob,
    md4 blob,
    md5 blob,
    sha1 blob,
    sha224 blob,
    sha256 blob,
    sha384 blob,
    sha512 blob,
    sha3_224 blob,
    sha3_256 blob,
    sha3_384 blob,
    sha3_512 blob,
    keccak224 blob,
    keccak256 blob,
    keccak384 blob,
    keccak512 blob,
    blake2s256 blob,
    blake2b512 blob,
    belt_hash blob,
    whirlpool blob,
    tiger blob,
    tiger2 blob,
    streebog256 blob,
    streebog512 blob,
    ripemd128 blob,
    ripemd160 blob,
    ripemd256 blob,
    ripemd320 blob,
    fsb160 blob,
    fsb224 blob,
    fsb256 blob,
    fsb384 blob,
    fsb512 blob,
    sm3 blob,
    gost94_cryptopro blob,
    gost94_test blob,
    gost94_ua blob,
    gost94_s2015 blob,
    groestl224 blob,
    groestl256 blob,
    groestl384 blob,
    groestl512 blob,
    shabal192 blob,
    shabal224 blob,
    shabal256 blob,
    shabal384 blob,
    shabal512 blob
);";

pub async fn init_database(db_string: &str, table_name: &str, use_wal: bool) -> Result<(), Error> {
    info!("Initializing SQLite database;");
    let db_path = db_string.trim_start_matches("sqlite://");

    let connection_options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true);
    let mut db_conn = SqliteConnection::connect_with(&connection_options)
        .await
        .expect("Failed to connect to db!");

    if use_wal {
        sqlx::query("PRAGMA journal_mode=WAL")
            .execute(&mut db_conn)
            .await
            .expect("Failed to enable WAL mode!");
        info!("Enabled WAL for database.");
    }

    let mut query_builder = QueryBuilder::new("CREATE TABLE IF NOT EXISTS ");
    query_builder.push(table_name);
    query_builder.push(HASHES);

    let query = query_builder.build();
    query
        .execute(&mut db_conn)
        .await
        .expect("Failed to create table!");

    info!("Wrote table with name {} to database.", table_name);

    Ok(())
}

pub async fn close_database(db_string: &str) {
    if let Ok(mut db_conn) = SqliteConnection::connect(db_string).await {
        let _ = sqlx::query("PRAGMA journal_mode=DELETE")
            .execute(&mut db_conn)
            .await;
    }
}

pub async fn get_file_hashes(
    path: &Path,
    conn: &mut SqliteConnection,
) -> Result<Vec<(String, (usize, Vec<u8>))>, Error> {
    let row: sqlx::sqlite::SqliteRow = sqlx::query("SELECT * FROM hashes WHERE file_path = ?")
        .bind(path.display().to_string())
        .fetch_optional(conn)
        .await?
        .ok_or_else(|| Error::Config("File not found in database".into()))?;

    let size = row.get::<i64, _>("file_size") as usize;
    let mut results = Vec::new();

    for col in row.columns() {
        let name = col.name();
        if name != "file_path" && name != "file_size" {
            if let Ok(hash) = row.try_get::<Option<Vec<u8>>, _>(name) {
                if let Some(hash) = hash {
                    if !hash.is_empty() {
                        results.push((name.to_string(), (size, hash)));
                    }
                }
            }
        }
    }

    Ok(results)
}

pub async fn get_all_paths(conn: &mut SqliteConnection) -> Result<Vec<PathBuf>, Error> {
    let query = "SELECT file_path FROM hashes";

    let rows = sqlx::query(query)
        .fetch_all(conn)
        .await?;

    Ok(rows.iter()
        .map(|row| PathBuf::from(row.get::<String, _>("file_path")))
        .collect())
}

pub async fn insert_single_hash(
    config: &Config,
    file_path: &Path,
    size: usize,
    hashes: &[(&str, Vec<u8>)],
    db_conn: &mut SqliteConnection,
) -> Result<(), Error> {
    let mut retries = 0;
    loop {
        let mut query_builder: QueryBuilder<sqlx::Sqlite> = QueryBuilder::new("INSERT INTO ");
        query_builder.push(&config.database.table_name);

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

        let query = query_builder.build();

        match query.execute(&mut *db_conn).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                if let Some(db_err) = e.as_database_error() {
                    if db_err.code().as_deref() == Some("SQLITE_BUSY") && retries < MAX_DB_RETRIES {
                        retries += 1;
                        sleep(DB_RETRY_DELAY).await;
                        continue;
                    }
                }
                return Err(Error::from(e));
            }
        }
    }
}
