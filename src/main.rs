use data_encoding::HEXLOWER;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read, Result};
use std::path::PathBuf;

/// calculates sha256 digest as lowercase hex string
fn sha256_digest(path: &PathBuf) -> Result<String> {
    let input = File::open(path)?;
    let mut reader = BufReader::new(input);

    let digest = {
        let mut hasher = Sha256::new();
        let mut buffer = [0; 1024];

        loop {
            let count = reader.read(&mut buffer)?;

            if count == 0 { break }

            hasher.update(&buffer[..count]);
        }

        hasher.finalize()
    };

    Ok(HEXLOWER.encode(digest.as_ref()))
}

fn main() {
    let file_name = "./test.tar.gz";
    let file_path = PathBuf::from(file_name);

    if let Ok(result) = sha256_digest(&file_path) {
        println!("{}", result);
    }
}
