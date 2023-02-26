use std::{
    fs,
    sync::{Arc, Mutex},
};

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
    /// The path to be hashed
    #[arg(short, long, default_value_t = String::from("."))]
    pub input_path: String,

    #[clap(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    /// Whether or not to write hashes to JSON
    #[arg(long, default_value_t = false)]
    pub json_out: bool,

    /// Whether or not to write hashes to the configured SQL database
    #[arg(long, default_value_t = false)]
    pub sql_out: bool,

    /// The path to output {path}/{sha256}.json
    #[arg(short, long, default_value_t = String::from("./hashes"))]
    pub json_output_path: String,

    /// The location of the config file
    #[arg(short, long, default_value_t = String::from("./config.toml"))]
    pub config_file: String,

    /// Reads file contents from stdin instead of any paths. --input-path becomes the path given in the output.
    /// Note: input must be smaller than the avaliable RAM.
    #[arg(long, default_value_t = false)]
    pub stdin: bool,

    /// Maximum number of subdirectories to descend when recursing directories
    #[arg(long, default_value_t = 16)]
    pub max_depth: usize,

    /// Number of files (inclusive) to skip before beginning to hash a directory.
    #[arg(long, default_value_t = 0)]
    pub skip_files: usize,

    /// DON'T follow symlinks
    #[arg(long, default_value_t = false)]
    pub no_follow_symlinks: bool,

    /// Hash directories breadth first instead of depth first
    #[arg(long, default_value_t = false)]
    pub breadth_first: bool,
}

#[derive(Deserialize)]
pub struct Hashes {
    pub crc32: bool,
    md2: bool,
    md4: bool,
    md5: bool,
    sha1: bool,
    sha224: bool,
    sha256: bool,
    sha384: bool,
    sha512: bool,
    sha3_224: bool,
    sha3_256: bool,
    sha3_384: bool,
    sha3_512: bool,
    keccak224: bool,
    keccak256: bool,
    keccak384: bool,
    keccak512: bool,
    blake2s256: bool,
    blake2b512: bool,
    belt_hash: bool,
    whirlpool: bool,
    tiger: bool,
    tiger2: bool,
    streebog256: bool,
    streebog512: bool,
    ripemd128: bool,
    ripemd160: bool,
    ripemd256: bool,
    ripemd320: bool,
    fsb160: bool,
    fsb224: bool,
    fsb256: bool,
    fsb384: bool,
    fsb512: bool,
    sm3: bool,
    gost94_cryptopro: bool,
    gost94_test: bool,
    gost94_ua: bool,
    gost94_s2015: bool,
    groestl224: bool,
    groestl256: bool,
    groestl384: bool,
    groestl512: bool,
    shabal192: bool,
    shabal224: bool,
    shabal256: bool,
    shabal384: bool,
    shabal512: bool,
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
                if config_hashes.$hash {
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
