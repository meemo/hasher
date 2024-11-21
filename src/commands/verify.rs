use std::path::Path;

use log::{error, info, warn};
use serde_json::Value;
use sqlx::Connection;

use crate::configuration::{Config, HasherVerifyArgs};
use crate::database::{get_all_paths, get_file_hashes};
use crate::utils::Error;
use hasher::{HashConfig, Hasher};

fn extract_stored_hashes(
    stored_hashes: &[(String, (usize, Vec<u8>))],
) -> (bool, bool, Vec<u8>, Vec<u8>, usize) {
    let stored_size = stored_hashes
        .first()
        .map(|(_, (size, _))| *size)
        .unwrap_or_default();
    let mut found_crc32 = false;
    let mut found_sha256 = false;
    let mut stored_crc32 = Vec::new();
    let mut stored_sha256 = Vec::new();

    for (name, (_, hash)) in stored_hashes {
        match name.as_str() {
            "crc32" => {
                found_crc32 = true;
                stored_crc32 = hash.clone();
            }
            "sha256" => {
                found_sha256 = true;
                stored_sha256 = hash.clone();
            }
            _ => {}
        }
    }

    (
        found_crc32,
        found_sha256,
        stored_crc32,
        stored_sha256,
        stored_size,
    )
}

fn extract_current_hashes(current_hashes: &[(&str, Vec<u8>)]) -> (Vec<u8>, Vec<u8>) {
    let mut current_crc32 = Vec::new();
    let mut current_sha256 = Vec::new();

    for (name, hash) in current_hashes {
        match *name {
            "crc32" => current_crc32 = hash.clone(),
            "sha256" => current_sha256 = hash.clone(),
            _ => {}
        }
    }

    (current_crc32, current_sha256)
}

fn validate_hashes(
    found_crc32: bool,
    found_sha256: bool,
    current_crc32: &[u8],
    current_sha256: &[u8],
    stored_crc32: &[u8],
    stored_sha256: &[u8],
) -> Option<(String, Vec<u8>, Vec<u8>)> {
    if !found_crc32 || !found_sha256 {
        return None;
    }

    if current_crc32 != stored_crc32 {
        return Some((
            "crc32".to_string(),
            current_crc32.to_vec(),
            stored_crc32.to_vec(),
        ));
    }

    if current_sha256 != stored_sha256 {
        return Some((
            "sha256".to_string(),
            current_sha256.to_vec(),
            stored_sha256.to_vec(),
        ));
    }

    None
}

fn build_verification_json(
    path: &Path,
    current_size: Option<usize>,
    stored_size: usize,
    failed_hash: Option<(String, Vec<u8>, Vec<u8>)>,
    is_missing: bool,
) -> String {
    let is_valid = failed_hash.is_none() && !is_missing;
    let path_str = path.display().to_string();

    let stored_hash = failed_hash
        .as_ref()
        .map(|(_, _, stored)| hex::encode(stored))
        .unwrap_or_else(|| hex::encode(&Vec::new()));

    // This is ugly because I want the output to be in a consistent order
    let (current_part, algorithm_part) = if is_missing {
        (
            format!(
                r#""current":{{"path":"{}","size":{},"hash":"file not found"}}"#,
                path_str, stored_size
            ),
            r#","algorithm":"file not found""#.to_string(),
        )
    } else if let Some((ref algorithm, ref current, _)) = failed_hash {
        (
            format!(
                r#""current":{{"path":"{}","size":{},"hash":"{}"}}"#,
                path_str,
                current_size.unwrap_or_default(),
                hex::encode(current)
            ),
            format!(r#","algorithm":"{}""#, algorithm),
        )
    } else {
        (String::new(), String::new())
    };

    format!(
        r#"{{"valid":{},"original":{{"path":"{}","size":{},"hash":"{}"}}{}{}}}"#,
        is_valid,
        path_str,
        stored_size,
        stored_hash,
        if !current_part.is_empty() {
            format!(",{}", current_part)
        } else {
            String::new()
        },
        algorithm_part
    )
}

async fn verify_file(
    path: &Path,
    args: &HasherVerifyArgs,
    db_conn: &mut sqlx::SqliteConnection,
) -> Result<(), Error> {
    let stored_hashes = get_file_hashes(path, db_conn).await?;
    let (found_crc32, found_sha256, stored_crc32, stored_sha256, stored_size) =
        extract_stored_hashes(&stored_hashes);

    if !found_crc32 || !found_sha256 {
        warn!("Missing required hashes for {}", path.display());
        return Ok(());
    }

    let (current_size, current_hashes) = if !path.exists() {
        info!("File not found: {}", path.display());
        (None, (Vec::new(), Vec::new()))
    } else {
        let mut hasher = Hasher::new(HashConfig {
            crc32: true,
            sha256: true,
            ..Default::default()
        });
        info!("Verifying {}", path.display());
        let (size, hashes) = hasher.hash_file(path)?;
        (Some(size), extract_current_hashes(&hashes))
    };

    let (current_crc32, current_sha256) = current_hashes;
    let failed_hash = if current_size.is_none() {
        // Use CRC32 for file not found case
        Some(("crc32".to_string(), Vec::new(), stored_crc32))
    } else {
        validate_hashes(
            found_crc32,
            found_sha256,
            &current_crc32,
            &current_sha256,
            &stored_crc32,
            &stored_sha256,
        )
    };

    if failed_hash.is_some() || current_size.is_none() || !args.mismatches_only {
        let output = build_verification_json(
            path,
            current_size,
            stored_size,
            failed_hash,
            current_size.is_none(),
        );

        if args.hash_options.pretty_json {
            if let Ok(parsed) = serde_json::from_str::<Value>(&output) {
                println!("{}", serde_json::to_string_pretty(&parsed).unwrap());
            } else {
                println!("{}", output);
            }
        } else {
            println!("{}", output);
        }
    }

    Ok(())
}

pub async fn execute(args: HasherVerifyArgs, config: &Config) -> Result<(), Error> {
    info!("Starting verification");

    let mut db_conn = sqlx::SqliteConnection::connect(&config.database.db_string).await?;
    let mut processed_count = 0;
    let mut missing_count = 0;
    let mut mismatch_count = 0;
    let mut error_count = 0;

    let paths = get_all_paths(&mut db_conn).await?;

    for path in paths {
        match verify_file(&path, &args, &mut db_conn).await {
            Ok(()) => {
                processed_count += 1;
                if !path.exists() {
                    missing_count += 1;
                } else {
                    // Check if file was mismatched by re-reading it to validate
                    if let Ok(stored_hashes) = get_file_hashes(&path, &mut db_conn).await {
                        let (found_crc32, found_sha256, stored_crc32, stored_sha256, _) =
                            extract_stored_hashes(&stored_hashes);
                        let mut hasher = Hasher::new(HashConfig {
                            crc32: true,
                            sha256: true,
                            ..Default::default()
                        });
                        if let Ok((_, hashes)) = hasher.hash_file(&path) {
                            let (current_crc32, current_sha256) = extract_current_hashes(&hashes);
                            if validate_hashes(
                                found_crc32,
                                found_sha256,
                                &current_crc32,
                                &current_sha256,
                                &stored_crc32,
                                &stored_sha256,
                            )
                            .is_some()
                            {
                                mismatch_count += 1;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                let err_msg = format!("Failed to verify {}: {}", path.display(), e);
                error_count += 1;
                if args.hash_options.continue_on_error {
                    error!("{}", err_msg);
                    continue;
                }
                return Err(e);
            }
        }
    }

    info!(
        "Processed {} files: {} missing, {} mismatched, {} errors",
        processed_count, missing_count, mismatch_count, error_count
    );

    Ok(())
}
