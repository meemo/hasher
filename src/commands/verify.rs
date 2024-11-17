use std::collections::HashMap;
use std::path::Path;
use sqlx::{Connection, SqliteConnection};
use log::{error, info};
use serde_json::json;

use crate::configuration::{HasherVerifyArgs, Config};
use crate::utils::Error;
use crate::database;
use hasher::{Hasher, HashConfig};

fn output_verification_json(
    path: &Path,
    current: Option<(usize, Vec<u8>)>,
    original: Option<(usize, Vec<u8>)>,
    algorithm: &str
) {
    let json = json!({
        "path": path.display().to_string(),
        "current": current.map(|(size, hash)| json!({
            "size": size,
            "hash": hex::encode(hash)
        })),
        "original": original.map(|(size, hash)| json!({
            "size": size,
            "hash": hex::encode(hash)
        })),
        "algorithm": algorithm
    });
    println!("{}", serde_json::to_string(&json).unwrap());
}

async fn verify_file(
    path: &Path,
    config: &Config,
    db_conn: &mut SqliteConnection,
    all_results: bool
) -> Result<bool, Error> {
    let stored = match database::get_file_hashes(path, db_conn).await {
        Ok(hashes) => hashes,
        Err(Error::Config(_)) => {
            error!("File not found in database: {}", path.display());
            return Ok(false);
        }
        Err(e) => return Err(e),
    };

    if stored.is_empty() {
        error!("No hashes found for file: {}", path.display());
        return Ok(false);
    }

    let (size, hashes) = match Hasher::new(HashConfig::from(&config.hashes)).hash_file(path) {
        Ok(result) => result,
        Err(e) => {
            error!("Failed to hash file {}: {}", path.display(), e);
            return Ok(false);
        }
    };

    let mut matches = true;
    let current_hashes: HashMap<String, _> = hashes.into_iter()
        .map(|(name, hash)| (name.to_string(), hash))
        .collect();

    for (algo, (stored_size, stored_hash)) in stored {
        if let Some(current_hash) = current_hashes.get(&algo) {
            if size != stored_size || current_hash.as_slice() != stored_hash.as_slice() {
                matches = false;
                output_verification_json(
                    path,
                    Some((size, current_hash.clone())),
                    Some((stored_size, stored_hash)),
                    &algo
                );
            } else if all_results {
                output_verification_json(
                    path,
                    Some((size, current_hash.clone())),
                    Some((stored_size, stored_hash)),
                    &algo
                );
            }
        }
    }

    Ok(matches)
}

pub async fn execute(args: HasherVerifyArgs, config: &Config) -> Result<(), Error> {
    let mut db_conn = SqliteConnection::connect(&config.database.db_string).await?;
    let mut total_files = 0;
    let mut mismatched_files = 0;

    for entry in walkdir::WalkDir::new(&args.source)
        .min_depth(0)
        .max_depth(args.max_depth)
        .follow_links(!args.no_follow_symlinks)
        .contents_first(!args.breadth_first)
        .sort_by_file_name()
    {
        let entry = entry?;
        if !entry.path().is_dir() {
            total_files += 1;
            if !verify_file(
                entry.path(),
                config,
                &mut db_conn,
                !args.mismatches_only
            ).await? {
                mismatched_files += 1;
            }
        }
    }

    info!(
        "Verified {} files, {} mismatches found",
        total_files, mismatched_files
    );
    Ok(())
}
