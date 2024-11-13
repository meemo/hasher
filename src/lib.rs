use std::io::{BufReader, Read};
use std::path::Path;
use std::fs::File;
use std::fmt;
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};

use digest::{Digest, DynDigest};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    ThreadPanic,
    FileChanged,
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
        }
    }
}

pub struct HashConfig {
    pub crc32: bool,
    pub md2: bool,
    pub md4: bool,
    pub md5: bool,
    pub sha1: bool,
    pub sha224: bool,
    pub sha256: bool,
    pub sha384: bool,
    pub sha512: bool,
    pub sha3_224: bool,
    pub sha3_256: bool,
    pub sha3_384: bool,
    pub sha3_512: bool,
    pub keccak224: bool,
    pub keccak256: bool,
    pub keccak384: bool,
    pub keccak512: bool,
    pub blake2s256: bool,
    pub blake2b512: bool,
    pub belt_hash: bool,
    pub whirlpool: bool,
    pub tiger: bool,
    pub tiger2: bool,
    pub streebog256: bool,
    pub streebog512: bool,
    pub ripemd128: bool,
    pub ripemd160: bool,
    pub ripemd256: bool,
    pub ripemd320: bool,
    pub fsb160: bool,
    pub fsb224: bool,
    pub fsb256: bool,
    pub fsb384: bool,
    pub fsb512: bool,
    pub sm3: bool,
    pub gost94_cryptopro: bool,
    pub gost94_test: bool,
    pub gost94_ua: bool,
    pub gost94_s2015: bool,
    pub groestl224: bool,
    pub groestl256: bool,
    pub groestl384: bool,
    pub groestl512: bool,
    pub shabal192: bool,
    pub shabal224: bool,
    pub shabal256: bool,
    pub shabal384: bool,
    pub shabal512: bool,
}

pub type HashResult = Vec<(&'static str, Vec<u8>)>;

const CHUNK_SIZE: usize = 512 * 1024 * 1024;
const SEQUENTIAL_SIZE: usize = 32 * 1024 * 1024;

pub struct Hasher {
    hashes: Vec<(&'static str, Arc<Mutex<Box<dyn DynDigest + Send>>>)>,
    crc32_hasher: Option<Arc<Mutex<crc32fast::Hasher>>>,
}

impl Hasher {
    pub fn new(config: HashConfig) -> Self {
        let mut hashes = Vec::new();
        let crc32_hasher = if config.crc32 {
            Some(Arc::new(Mutex::new(crc32fast::Hasher::new())))
        } else {
            None
        };

        macro_rules! init_hash {
            ($($name:ident, $type:ty),*) => {
                $(
                    if config.$name {
                        hashes.push((
                            stringify!($name),
                            Arc::new(Mutex::new(Box::new(<$type>::new()) as Box<dyn DynDigest + Send>))
                        ));
                    }
                )*
            }
        }

        init_hash!(
            md2, md2::Md2,
            md4, md4::Md4,
            md5, md5::Md5,
            sha1, sha1::Sha1,
            sha224, sha2::Sha224,
            sha256, sha2::Sha256,
            sha384, sha2::Sha384,
            sha512, sha2::Sha512,
            sha3_224, sha3::Sha3_224,
            sha3_256, sha3::Sha3_256,
            sha3_384, sha3::Sha3_384,
            sha3_512, sha3::Sha3_512,
            keccak224, sha3::Keccak224,
            keccak256, sha3::Keccak256,
            keccak384, sha3::Keccak384,
            keccak512, sha3::Keccak512,
            blake2s256, blake2::Blake2s256,
            blake2b512, blake2::Blake2b512,
            belt_hash, belt_hash::BeltHash,
            whirlpool, whirlpool::Whirlpool,
            tiger, tiger::Tiger,
            tiger2, tiger::Tiger2,
            streebog256, streebog::Streebog256,
            streebog512, streebog::Streebog512,
            ripemd128, ripemd::Ripemd128,
            ripemd160, ripemd::Ripemd160,
            ripemd256, ripemd::Ripemd256,
            ripemd320, ripemd::Ripemd320,
            fsb160, fsb::Fsb160,
            fsb224, fsb::Fsb224,
            fsb256, fsb::Fsb256,
            fsb384, fsb::Fsb384,
            fsb512, fsb::Fsb512,
            sm3, sm3::Sm3,
            gost94_cryptopro, gost94::Gost94CryptoPro,
            gost94_test, gost94::Gost94Test,
            gost94_ua, gost94::Gost94UA,
            gost94_s2015, gost94::Gost94s2015,
            groestl224, groestl::Groestl224,
            groestl256, groestl::Groestl256,
            groestl384, groestl::Groestl384,
            groestl512, groestl::Groestl512,
            shabal192, shabal::Shabal192,
            shabal224, shabal::Shabal224,
            shabal256, shabal::Shabal256,
            shabal384, shabal::Shabal384,
            shabal512, shabal::Shabal512
        );

        Self {
            hashes,
            crc32_hasher,
        }
    }

    fn finalize_hashes(&mut self) -> Result<HashResult, Error> {
        let mut results = Vec::new();

        if let Some(crc32) = &self.crc32_hasher {
            results.push((
                "crc32",
                crc32.lock().unwrap().clone().finalize().to_be_bytes().to_vec()
            ));
        }

        for (name, hasher) in &mut self.hashes {
            results.push((*name, hasher.lock().unwrap().finalize_reset().to_vec()));
        }

        Ok(results)
    }

    fn hash_buffer_sequential(&mut self, buffer: &Arc<RwLock<Vec<u8>>>) -> Result<(), Error> {
        let buffer_guard = buffer.read().map_err(|_| Error::ThreadPanic)?;

        for (_name, hasher) in &self.hashes {
            hasher.lock()
                .map_err(|_| Error::ThreadPanic)?
                .update(&buffer_guard);
        }

        if let Some(crc32) = &self.crc32_hasher {
            crc32.lock()
                .map_err(|_| Error::ThreadPanic)?
                .update(&buffer_guard);
        }

        Ok(())
    }

    fn hash_buffer_threaded(&mut self, buffer: &Arc<RwLock<Vec<u8>>>) -> Result<(), Error> {
        let mut threads: Vec<JoinHandle<Result<(), Error>>> = Vec::new();

        for (_name, hasher) in &self.hashes {
            let buffer = buffer.clone();
            let hasher = hasher.clone();

            threads.push(thread::spawn(move || {
                hasher.lock()
                    .map_err(|_| Error::ThreadPanic)?
                    .update(&buffer.read().map_err(|_| Error::ThreadPanic)?);
                Ok(())
            }));
        }

        if let Some(crc32) = &self.crc32_hasher {
            let buffer = buffer.clone();
            let hasher = crc32.clone();

            threads.push(thread::spawn(move || {
                hasher.lock()
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

    fn open_file(path: &Path) -> Result<(BufReader<File>, usize), Error> {
        let file = File::open(path)?;
        let size = file.metadata()?.len() as usize;
        Ok((BufReader::new(file), size))
    }

    pub fn hash_buffer(&mut self, buffer: &[u8]) -> Result<(), Error> {
        let buffer_arc = Arc::new(RwLock::new(buffer.to_vec()));

        if buffer.len() < SEQUENTIAL_SIZE {
            self.hash_buffer_sequential(&buffer_arc)
        } else {
            self.hash_buffer_threaded(&buffer_arc)
        }
    }

    pub fn hash_file(&mut self, path: &Path) -> Result<(usize, HashResult), Error> {
        let (mut reader, file_size) = Hasher::open_file(path)?;
        let mut buffer = vec![0; CHUNK_SIZE.min(file_size)];
        let start_metadata = path.metadata()?;

        loop {
            let bytes_read = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(Error::Io(e)),
            };

            // Check if file changed during read
            if let Ok(current_metadata) = path.metadata() {
                if current_metadata.modified()? != start_metadata.modified()? {
                    return Err(Error::FileChanged);
                }
            }

            buffer.truncate(bytes_read);
            self.hash_buffer(&buffer)?;
        }

        Ok((file_size, self.finalize_hashes()?))
    }

    // For single-buffer hashing convenience
    pub fn hash_single_buffer(&mut self, buffer: &[u8]) -> Result<HashResult, Error> {
        self.hash_buffer(buffer)?;
        self.finalize_hashes()
    }
}
