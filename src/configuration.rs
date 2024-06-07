use std::sync::{Arc, Mutex};
use std::fs;

use belt_hash::BeltHash;
use blake2::{Blake2b512, Blake2s256};
use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use digest::{Digest, DynDigest};
use fsb::{Fsb160, Fsb224, Fsb256, Fsb384, Fsb512};
use gost94::{Gost94CryptoPro, Gost94Test, Gost94UA, Gost94s2015};
use groestl::{Groestl224, Groestl256, Groestl384, Groestl512};
use md2::Md2;
use md4::Md4;
use md5::Md5;
use ripemd::{Ripemd128, Ripemd160, Ripemd256, Ripemd320};
use serde_derive::Deserialize;
use sha1::Sha1;
use sha2::{Sha224, Sha256, Sha384, Sha512};
use sha3::{Keccak224, Keccak256, Keccak384, Keccak512, Sha3_224, Sha3_256, Sha3_384, Sha3_512};
use shabal::{Shabal192, Shabal224, Shabal256, Shabal384, Shabal512};
use sm3::Sm3;
use streebog::{Streebog256, Streebog512};
use tiger::{Tiger, Tiger2};
use whirlpool::Whirlpool;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// The path to hash the files inside
    #[arg(short, long, default_value_t = String::from("."))]
    pub input_path: String,

    #[clap(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    /// Write hashes to JSON
    #[arg(long, default_value_t = false)]
    pub json_out: bool,

    /// Write hashes to the SQLite database in the config
    #[arg(long, default_value_t = false)]
    pub sql_out: bool,

    /// Enable WAL mode in the SQLite database while running
    #[arg(long, default_value_t = false)]
    pub use_wal: bool,

    /// The path to output {path}/{sha256 of file}.json
    #[arg(short, long, default_value_t = String::from("./hashes"))]
    pub json_output_path: String,

    /// The location of the config file
    #[arg(short, long, default_value_t = String::from("./config.toml"))]
    pub config_file: String,

    /// Reads file contents from stdin instead of any paths. --input-path becomes the path given in the output
    #[arg(long, default_value_t = false)]
    pub stdin: bool,

    /// Maximum number of subdirectories to descend when recursing directories
    #[arg(long, default_value_t = 20)]
    pub max_depth: usize,

    /// Number of files (inclusive) to skip before beginning to hash a directory.
    /// Meant for resuming interrupted hashing runs, don't use this normally.
    #[arg(long, default_value_t = 0)]
    pub skip_files: usize,

    /// DON'T follow symlinks. Infinite loops are possible if this is off and there are bad symlinks.
    #[arg(long, default_value_t = false)]
    pub no_follow_symlinks: bool,

    /// Hash directories breadth first instead of depth first
    #[arg(long, default_value_t = false)]
    pub breadth_first: bool,

    /// Does not write hashes anywhere but stdout. Useful for benchmarking and if you hands are cold.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct Hashes {
    pub crc32: Option<bool>,
    md2: Option<bool>,
    md4: Option<bool>,
    md5: Option<bool>,
    sha1: Option<bool>,
    sha224: Option<bool>,
    sha256: Option<bool>,
    sha384: Option<bool>,
    sha512: Option<bool>,
    sha3_224: Option<bool>,
    sha3_256: Option<bool>,
    sha3_384: Option<bool>,
    sha3_512: Option<bool>,
    keccak224: Option<bool>,
    keccak256: Option<bool>,
    keccak384: Option<bool>,
    keccak512: Option<bool>,
    blake2s256: Option<bool>,
    blake2b512: Option<bool>,
    belt_hash: Option<bool>,
    whirlpool: Option<bool>,
    tiger: Option<bool>,
    tiger2: Option<bool>,
    streebog256: Option<bool>,
    streebog512: Option<bool>,
    ripemd128: Option<bool>,
    ripemd160: Option<bool>,
    ripemd256: Option<bool>,
    ripemd320: Option<bool>,
    fsb160: Option<bool>,
    fsb224: Option<bool>,
    fsb256: Option<bool>,
    fsb384: Option<bool>,
    fsb512: Option<bool>,
    sm3: Option<bool>,
    gost94_cryptopro: Option<bool>,
    gost94_test: Option<bool>,
    gost94_ua: Option<bool>,
    gost94_s2015: Option<bool>,
    groestl224: Option<bool>,
    groestl256: Option<bool>,
    groestl384: Option<bool>,
    groestl512: Option<bool>,
    shabal192: Option<bool>,
    shabal224: Option<bool>,
    shabal256: Option<bool>,
    shabal384: Option<bool>,
    shabal512: Option<bool>,
}

#[derive(Deserialize)]
pub struct Database {
    pub db_string: String,
    pub table_name: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub database: Database,
    pub hashes: Hashes,
}

pub fn get_config(config_args: &Args) -> Config {
    if let Ok(config_str) = fs::read_to_string(&config_args.config_file) {
        let config: Config =
            toml::from_str(&config_str).expect("Fatal error when reading config file contents!");
        return config;
    } else {
        panic!("Failed to read config file at {}!", config_args.config_file);
    }
}

pub fn get_hashes<'a>(
    config_hashes: &Hashes,
) -> Arc<Mutex<Vec<(&'a str, Arc<Mutex<dyn DynDigest + Send>>)>>> {
    let hashes: Arc<Mutex<Vec<(&str, Arc<Mutex<dyn DynDigest + Send>>)>>> =
        Arc::new(Mutex::new(Vec::new()));

    macro_rules! addhashes {
        ( $(($hash:tt, $hash_fn:tt)),* ) => {
            $(
                if config_hashes.$hash.is_some() && config_hashes.$hash.unwrap() {
                    hashes
                    .lock()
                    .unwrap()
                    .push((stringify!($hash), Arc::new(Mutex::new($hash_fn::new()))))
                };
            )*
        }
    }

    addhashes!(
        (md2, Md2),
        (md4, Md4),
        (md5, Md5),
        (sha1, Sha1),
        (sha224, Sha224),
        (sha256, Sha256),
        (sha384, Sha384),
        (sha512, Sha512),
        (sha3_224, Sha3_224),
        (sha3_256, Sha3_256),
        (sha3_384, Sha3_384),
        (sha3_512, Sha3_512),
        (keccak224, Keccak224),
        (keccak256, Keccak256),
        (keccak384, Keccak384),
        (keccak512, Keccak512),
        (belt_hash, BeltHash),
        (blake2s256, Blake2s256),
        (blake2b512, Blake2b512),
        (whirlpool, Whirlpool),
        (tiger, Tiger),
        (tiger2, Tiger2),
        (streebog256, Streebog256),
        (streebog512, Streebog512),
        (ripemd128, Ripemd128),
        (ripemd160, Ripemd160),
        (ripemd256, Ripemd256),
        (ripemd320, Ripemd320),
        (ripemd128, Ripemd128),
        (fsb160, Fsb160),
        (fsb224, Fsb224),
        (fsb256, Fsb256),
        (fsb384, Fsb384),
        (fsb512, Fsb512),
        (sm3, Sm3),
        (gost94_cryptopro, Gost94CryptoPro),
        (gost94_test, Gost94Test),
        (gost94_ua, Gost94UA),
        (gost94_s2015, Gost94s2015),
        (groestl224, Groestl224),
        (groestl256, Groestl256),
        (groestl384, Groestl384),
        (groestl512, Groestl512),
        (shabal192, Shabal192),
        (shabal224, Shabal224),
        (shabal256, Shabal256),
        (shabal384, Shabal384),
        (shabal512, Shabal512)
    );

    hashes
}
