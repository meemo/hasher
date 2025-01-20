use std::fmt;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};

use digest::{Digest, DynDigest};
use log::info;

const CHUNK_SIZE: usize = 512 * 1024 * 1024; // 512 MiB
const SEQUENTIAL_SIZE: usize = 32 * 1024 * 1024; // 32 MiB
const MAX_FILE_SIZE: usize = usize::MAX;

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    ThreadPanic,
    FileChanged,
    InvalidInput(&'static str),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {}", e),
            Error::ThreadPanic => write!(f, "Thread panic occurred"),
            Error::FileChanged => write!(f, "File was modified during reading"),
            Error::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
        }
    }
}

macro_rules! define_hash_algorithms {
    ($(($name:ident, $type:ty)),*) => {
        #[derive(Default)]
        pub struct HashConfig {
            pub crc32: bool,
            $(pub $name: bool,)*
        }

        impl Hasher {
            pub fn new(config: HashConfig) -> Self {
                let mut hashes = Vec::new();
                let crc32_hasher = config.crc32.then(|| Arc::new(Mutex::new(crc32fast::Hasher::new())));

                $(
                    if config.$name {
                        hashes.push((
                            stringify!($name),
                            Arc::new(Mutex::new(Box::new(<$type>::new()) as Box<dyn DynDigest + Send>))
                        ));
                    }
                )*

                info!(
                    "Initialized hasher with {} algorithms",
                    hashes.len() + crc32_hasher.is_some() as usize
                );
                Self {
                    hashes,
                    crc32_hasher,
                }
            }
        }
    };
}

define_hash_algorithms!(
    (md2, md2::Md2),
    (md4, md4::Md4),
    (md5, md5::Md5),
    (sha1, sha1::Sha1),
    (sha224, sha2::Sha224),
    (sha256, sha2::Sha256),
    (sha384, sha2::Sha384),
    (sha512, sha2::Sha512),
    (sha3_224, sha3::Sha3_224),
    (sha3_256, sha3::Sha3_256),
    (sha3_384, sha3::Sha3_384),
    (sha3_512, sha3::Sha3_512),
    (keccak224, sha3::Keccak224),
    (keccak256, sha3::Keccak256),
    (keccak384, sha3::Keccak384),
    (keccak512, sha3::Keccak512),
    (blake2s256, blake2::Blake2s256),
    (blake2b512, blake2::Blake2b512),
    (belt_hash, belt_hash::BeltHash),
    (whirlpool, whirlpool::Whirlpool),
    (tiger, tiger::Tiger),
    (tiger2, tiger::Tiger2),
    (streebog256, streebog::Streebog256),
    (streebog512, streebog::Streebog512),
    (ripemd128, ripemd::Ripemd128),
    (ripemd160, ripemd::Ripemd160),
    (ripemd256, ripemd::Ripemd256),
    (ripemd320, ripemd::Ripemd320),
    (fsb160, fsb::Fsb160),
    (fsb224, fsb::Fsb224),
    (fsb256, fsb::Fsb256),
    (fsb384, fsb::Fsb384),
    (fsb512, fsb::Fsb512),
    (sm3, sm3::Sm3),
    (gost94_cryptopro, gost94::Gost94CryptoPro),
    (gost94_test, gost94::Gost94Test),
    (gost94_ua, gost94::Gost94UA),
    (gost94_s2015, gost94::Gost94s2015),
    (groestl224, groestl::Groestl224),
    (groestl256, groestl::Groestl256),
    (groestl384, groestl::Groestl384),
    (groestl512, groestl::Groestl512),
    (shabal192, shabal::Shabal192),
    (shabal224, shabal::Shabal224),
    (shabal256, shabal::Shabal256),
    (shabal384, shabal::Shabal384),
    (shabal512, shabal::Shabal512)
);

pub type HashResult = Vec<(&'static str, Vec<u8>)>;

pub struct Hasher {
    hashes: Vec<(&'static str, Arc<Mutex<Box<dyn DynDigest + Send>>>)>,
    crc32_hasher: Option<Arc<Mutex<crc32fast::Hasher>>>,
}

impl Hasher {
    fn finalize_hashes(&mut self) -> Result<HashResult, Error> {
        let mut results =
            Vec::with_capacity(self.hashes.len() + self.crc32_hasher.is_some() as usize);

        if let Some(crc32) = &self.crc32_hasher {
            results.push((
                "crc32",
                crc32
                    .lock()
                    .map_err(|_| Error::ThreadPanic)?
                    .clone()
                    .finalize()
                    .to_le_bytes()
                    .to_vec(),
            ));
        }

        for (name, hasher) in &mut self.hashes {
            results.push((
                *name,
                hasher
                    .lock()
                    .map_err(|_| Error::ThreadPanic)?
                    .finalize_reset()
                    .to_vec(),
            ));
        }

        Ok(results)
    }

    fn validate_file(path: &Path) -> Result<(BufReader<File>, usize), Error> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;

        if !metadata.is_file() {
            return Err(Error::InvalidInput("Not a regular file"));
        }

        if metadata.len() as usize > MAX_FILE_SIZE {
            return Err(Error::InvalidInput("File too large"));
        }

        let size = metadata.len() as usize;
        info!("Opened file {} ({} bytes)", path.display(), size);
        Ok((BufReader::with_capacity(CHUNK_SIZE.min(size), file), size))
    }

    fn hash_buffer_sequential(&mut self, buffer: &Arc<RwLock<Vec<u8>>>) -> Result<(), Error> {
        let guard = buffer.read().map_err(|_| Error::ThreadPanic)?;

        for (_name, hasher) in &self.hashes {
            hasher
                .lock()
                .map_err(|_| Error::ThreadPanic)?
                .update(&guard);
        }

        if let Some(crc32) = &self.crc32_hasher {
            crc32.lock().map_err(|_| Error::ThreadPanic)?.update(&guard);
        }

        Ok(())
    }

    fn hash_buffer_threaded(&mut self, buffer: &Arc<RwLock<Vec<u8>>>) -> Result<(), Error> {
        let mut threads: Vec<JoinHandle<Result<(), Error>>> =
            Vec::with_capacity(self.hashes.len() + self.crc32_hasher.is_some() as usize);

        for (_name, hasher) in &self.hashes {
            let buffer = Arc::clone(buffer);
            let hasher = Arc::clone(hasher);

            threads.push(thread::spawn(move || {
                hasher
                    .lock()
                    .map_err(|_| Error::ThreadPanic)?
                    .update(&buffer.read().map_err(|_| Error::ThreadPanic)?);
                Ok(())
            }));
        }

        if let Some(crc32) = &self.crc32_hasher {
            let buffer = Arc::clone(buffer);
            let hasher = Arc::clone(crc32);

            threads.push(thread::spawn(move || {
                hasher
                    .lock()
                    .map_err(|_| Error::ThreadPanic)?
                    .update(&buffer.read().map_err(|_| Error::ThreadPanic)?);
                Ok(())
            }));
        }

        for handle in threads {
            handle.join().map_err(|_| Error::ThreadPanic)??;
        }

        Ok(())
    }

    pub fn hash_buffer(&mut self, buffer: &[u8]) -> Result<(), Error> {
        if buffer.is_empty() {
            return Ok(());
        }

        let buffer_arc = Arc::new(RwLock::new(buffer.to_vec()));

        if buffer.len() < SEQUENTIAL_SIZE {
            self.hash_buffer_sequential(&buffer_arc)
        } else {
            info!("Processing {} byte chunk in parallel", buffer.len());
            self.hash_buffer_threaded(&buffer_arc)
        }
    }

    pub fn hash_file(&mut self, path: &Path) -> Result<(usize, HashResult), Error> {
        let (mut reader, file_size) = Self::validate_file(path)?;
        let mut buffer = vec![0; CHUNK_SIZE.min(file_size)];
        let start_metadata = path.metadata()?;

        loop {
            let bytes_read = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(Error::Io(e)),
            };

            if let Ok(current_metadata) = path.metadata() {
                if current_metadata.modified()? != start_metadata.modified()? {
                    info!("File {} was modified during processing", path.display());
                    return Err(Error::FileChanged);
                }
            }

            buffer.truncate(bytes_read);
            self.hash_buffer(&buffer)?;
        }

        let results = self.finalize_hashes()?;
        info!(
            "Completed hashing of {} with {} algorithms",
            path.display(),
            results.len()
        );
        Ok((file_size, results))
    }

    pub fn hash_single_buffer(&mut self, buffer: &[u8]) -> Result<HashResult, Error> {
        self.hash_buffer(buffer)?;
        self.finalize_hashes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_hash_empty_buffer() {
        let mut hasher = Hasher::new(HashConfig {
            sha256: true,
            ..Default::default()
        });
        assert!(hasher.hash_buffer(&[]).is_ok());
    }

    #[test]
    fn test_hash_small_buffer() {
        let data = b"test data";
        let mut hasher = Hasher::new(HashConfig {
            sha256: true,
            ..Default::default()
        });
        assert!(hasher.hash_buffer(data).is_ok());
    }

    #[test]
    fn test_multiple_hashes() {
        let data = b"test data for multiple hashes";
        let mut hasher = Hasher::new(HashConfig {
            md5: true,
            sha1: true,
            sha256: true,
            ..Default::default()
        });
        let result = hasher.hash_single_buffer(data).unwrap();
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn test_hash_file() {
        let mut file = NamedTempFile::new().unwrap();
        let content = b"test file content";
        file.write_all(content).unwrap();
        file.flush().unwrap();

        let mut hasher = Hasher::new(HashConfig {
            sha256: true,
            ..Default::default()
        });
        let (size, hashes) = hasher.hash_file(file.path()).unwrap();

        assert_eq!(size, content.len());
        assert_eq!(hashes.len(), 1);
    }

    #[test]
    fn test_file_modified_during_read() {
        const TEST_FILE_SIZE: usize = 256 * 1024;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(&vec![b'a'; TEST_FILE_SIZE]).unwrap();
        file.flush().unwrap();

        let path = file.path().to_path_buf();
        let path_clone = path.clone();

        let handle = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(10));
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .append(true)
                .open(&path_clone)
                .unwrap();
            file.write_all(b"modified content").unwrap();
            file.flush().unwrap();
            file.sync_all().unwrap();
        });

        std::thread::sleep(std::time::Duration::from_millis(5));

        let mut hasher = Hasher::new(HashConfig {
            sha256: true,
            ..Default::default()
        });

        match hasher.hash_file(&path) {
            Err(Error::FileChanged) => (),
            other => panic!("Expected FileChanged error, got {:?}", other),
        }

        handle.join().unwrap();
    }

    #[test]
    fn test_hash_nonexistent_file() {
        let mut hasher = Hasher::new(HashConfig {
            sha256: true,
            ..Default::default()
        });
        assert!(matches!(
            hasher.hash_file(Path::new("nonexistent.txt")),
            Err(Error::Io(_))
        ));
    }
}
