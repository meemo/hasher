use sqlx::{query_builder::QueryBuilder, Connection};
use sqlx::{SqliteConnection, sqlite::SqliteConnectOptions};
use log::info;

use crate::utils::Error;

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
