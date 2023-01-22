use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::env;
use std::process;

use blake2::Blake2b512;
use digest::{Digest, DynDigest};
use md5::Md5;
use sha1::Sha1;
use sha2::{Sha224, Sha256, Sha384, Sha512};
use hex;

mod hasher;
use hasher::hash_file_threaded;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Invalid number of args ({}, expected 1)", args.len() - 1);
        process::exit(1);
    }

    let mut hashes: Vec<(&str, Arc<Mutex<dyn DynDigest + Send>>)> = Vec::new();
    hashes.push(("md5", Arc::new(Mutex::new(Md5::new()))));
    hashes.push(("sha1", Arc::new(Mutex::new(Sha1::new()))));
    hashes.push(("sha224", Arc::new(Mutex::new(Sha224::new()))));
    hashes.push(("sha256", Arc::new(Mutex::new(Sha256::new()))));
    hashes.push(("sha384", Arc::new(Mutex::new(Sha384::new()))));
    hashes.push(("sha512", Arc::new(Mutex::new(Sha512::new()))));
    hashes.push(("blake2b512", Arc::new(Mutex::new(Blake2b512::new()))));

    if let Ok(results) = hash_file_threaded(&PathBuf::from(&args[1]), &mut hashes, true) {
        for result in results {
            println!("{}: {}", result.0, hex::encode(result.1));
        }
    } else {
        println!("Failure");
    }
}
