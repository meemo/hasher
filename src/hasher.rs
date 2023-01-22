use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::thread::JoinHandle;
use std::cmp::min;

use crc32fast;
use digest::DynDigest;

// Read 1 GiB of the file at a time when dealing with large files
const CHUNK_SIZE: usize = 1024 * 1024 * 1024;

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

// Where the magic happens
pub fn hash_file_threaded<'a>(
    file_path: &'a PathBuf,
    hashes: &'a mut Vec<(&str, Arc<Mutex<dyn DynDigest + Send>>)>,
    hash_crc32: bool,
) -> Result<Vec<(&'a str, Vec<u8>)>, Error> {
    let input_file = File::open(file_path)?;
    let file_size: usize = input_file.metadata()?.len() as usize;
    let mut file_reader = BufReader::new(input_file);

    let buffer_size: usize = min(file_size, CHUNK_SIZE);
    let buffer = Arc::new(RwLock::new(vec![0; buffer_size]));

    let crc32_hasher = Arc::new(Mutex::new(crc32fast::Hasher::new()));

    // Hash the entire buffer
    loop {
        let mut threads: Vec<JoinHandle<Result<(), Error>>> = Vec::new();

        let bytes_read = file_reader.read(buffer.write()?.as_mut())?;

        if bytes_read == 0 { break; }

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
        if hash_crc32 {
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
    if hash_crc32 {
        final_hashes.push(("crc32", crc32_hasher.lock().unwrap().clone().finalize().to_be_bytes().to_vec()));
    }

    for (hash_name, hash_mutex) in hashes.drain(..) {
        final_hashes.push((hash_name, hash_mutex.lock().unwrap().finalize_reset().to_vec()));
    }

    Ok(final_hashes)
}
