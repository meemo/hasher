use std::env;
use std::process;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::thread;
use std::thread::JoinHandle;
use std::sync::{Arc, Mutex, RwLock};

use data_encoding::HEXLOWER;
use digest::{Digest, DynDigest};
use md5::Md5;
use sha1::Sha1;
use sha2::{Sha224, Sha256, Sha384, Sha512};
use blake2::Blake2b512;

// Read 1 GiB of the file at a time.
const CHUNK_SIZE: usize = 1024 * 1024 * 1024;

enum Error {
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

// Where the magic happens
fn hash_file(file_path: &PathBuf) -> Result<String, Error> {
    let input_file = File::open(file_path)?;
    let file_size: usize = input_file.metadata()?.len() as usize;
    let mut file_reader = BufReader::new(input_file);

    // Don't waste RAM on small files
    let buffer_size: usize = if file_size < CHUNK_SIZE { file_size } else { CHUNK_SIZE };
    let buffer = Arc::new(RwLock::new(vec![0; buffer_size]));

    // Please lord, forgive me for my sins
    let mut hashes: Vec<(&str, Arc<Mutex<dyn DynDigest + Send + Sync>>)> = Vec::new();
    hashes.push(("MD5", Arc::new(Mutex::new(Md5::new()))));
    hashes.push(("SHA-1", Arc::new(Mutex::new(Sha1::new()))));
    hashes.push(("SHA-224", Arc::new(Mutex::new(Sha224::new()))));
    hashes.push(("SHA-256", Arc::new(Mutex::new(Sha256::new()))));
    hashes.push(("SHA-384", Arc::new(Mutex::new(Sha384::new()))));
    hashes.push(("SHA-512", Arc::new(Mutex::new(Sha512::new()))));
    hashes.push(("Blake2b512", Arc::new(Mutex::new(Blake2b512::new()))));

    loop {
        let mut threads: Vec<JoinHandle<Result<(), Error>>> = Vec::new();

        let bytes_read = file_reader.read(buffer.write()?.as_mut())?;

        // Note: It is fine if hashes are never updated (i.e. zero byte files)
        if bytes_read == 0 {
            break
        }

        // On the final iteration resize the vector to the actual amount of data read
        if bytes_read < buffer_size {
            buffer.write()?.resize(bytes_read, 0);
        }

        // Launch threads for each hash
        for (_hash_name, hasher) in hashes.iter_mut() {
            let current_buf = buffer.clone();
            let hasher_rc = hasher.clone();

            threads.push(thread::spawn(move || {
                let mut hasher_lock = hasher_rc.lock()?;
                hasher_lock.update(current_buf.read()?.as_slice());
                Ok(())
            }));
        }

        // Wait for all threads to finish processing
        for handle in threads.into_iter() {
            handle.join().unwrap()?;
        }
    }

    for (hash_name, hash_mutex) in hashes.drain(..) {
        let mut hasher_lock = hash_mutex.lock().unwrap();
        println!("{}: {}", hash_name, HEXLOWER.encode(&hasher_lock.finalize_reset()));
    }

    Ok("Done".to_string())
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Invalid number of args ({}, expected 1)", args.len() - 1);
        process::exit(1);
    }

    if let Ok(result) = hash_file(&PathBuf::from(&args[1])) {
        println!("{}", result);
    }
}
