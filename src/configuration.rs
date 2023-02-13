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

/**
 * configuration.rs
 *
 * Configuration of the program.
 */

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// The path to be hashed
    #[arg(short, long, default_value_t = String::from("."))]
    pub input_path: String,

    #[clap(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    /// The path to output hashes, {path}/{sha256}.json
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

macro_rules! addhash {
    ($hashes:ident, $name:literal, $hash:expr) => {
        $hashes
            .lock()
            .unwrap()
            .push(($name, Arc::new(Mutex::new($hash))))
    };
}

// Please don't look at this.
#[rustfmt::skip]
pub fn get_hashes<'a>(
    config_hashes: &Hashes,
) -> Arc<Mutex<Vec<(&'a str, Arc<Mutex<dyn DynDigest + Send>>)>>> {
    let hashes: Arc<Mutex<Vec<(&str, Arc<Mutex<dyn DynDigest + Send>>)>>> =
        Arc::new(Mutex::new(Vec::new()));

    if config_hashes.md2 { addhash!(hashes, "md2", Md2::new()); }
    if config_hashes.md4 { addhash!(hashes, "md4", Md4::new()); }
    if config_hashes.md5 { addhash!(hashes, "md5", Md5::new()); }
    if config_hashes.sha1 { addhash!(hashes, "sha1", Sha1::new()); }
    if config_hashes.sha224 { addhash!(hashes, "sha224", Sha224::new()); }
    if config_hashes.sha256 { addhash!(hashes, "sha256", Sha256::new()); }
    if config_hashes.sha384 { addhash!(hashes, "sha384", Sha384::new()); }
    if config_hashes.sha512 { addhash!(hashes, "sha512", Sha512::new()); }
    if config_hashes.sha3_224 { addhash!(hashes, "sha3_224", Sha3_224::new()); }
    if config_hashes.sha3_256 { addhash!(hashes, "sha3_256", Sha3_256::new()); }
    if config_hashes.sha3_384 { addhash!(hashes, "sha3_384", Sha3_384::new()); }
    if config_hashes.sha3_512 { addhash!(hashes, "sha3_512", Sha3_512::new()); }
    if config_hashes.keccak224 { addhash!(hashes, "keccak224", Keccak224::new()); }
    if config_hashes.keccak256 { addhash!(hashes, "keccak256", Keccak256::new()); }
    if config_hashes.keccak384 { addhash!(hashes, "keccak384", Keccak384::new()); }
    if config_hashes.keccak512 { addhash!(hashes, "keccak512", Keccak512::new()); }
    if config_hashes.belt_hash { addhash!(hashes, "belt_hash", BeltHash::new()); }
    if config_hashes.blake2s256 { addhash!(hashes, "blake2s256", Blake2s256::new()); }
    if config_hashes.blake2b512 { addhash!(hashes, "blake2b512", Blake2b512::new()); }
    if config_hashes.whirlpool { addhash!(hashes, "whirlpool", Whirlpool::new()); }
    if config_hashes.tiger { addhash!(hashes, "tiger", Tiger::new()); }
    if config_hashes.tiger2 { addhash!(hashes, "tiger2", Tiger2::new()); }
    if config_hashes.streebog256 { addhash!(hashes, "streebog256", Streebog256::new()); }
    if config_hashes.streebog512 { addhash!(hashes, "streebog512", Streebog512::new()); }
    if config_hashes.ripemd128 { addhash!(hashes, "ripemd128", Ripemd128::new()); }
    if config_hashes.ripemd160 { addhash!(hashes, "ripemd160", Ripemd160::new()); }
    if config_hashes.ripemd256 { addhash!(hashes, "ripemd256", Ripemd256::new()); }
    if config_hashes.ripemd320 { addhash!(hashes, "ripemd320", Ripemd320::new()); }
    if config_hashes.ripemd128 { addhash!(hashes, "ripemd128", Ripemd128::new()); }
    if config_hashes.fsb160 { addhash!(hashes, "fsb160", Fsb160::new()); }
    if config_hashes.fsb224 { addhash!(hashes, "fsb224", Fsb224::new()); }
    if config_hashes.fsb256 { addhash!(hashes, "fsb256", Fsb256::new()); }
    if config_hashes.fsb384 { addhash!(hashes, "fsb384", Fsb384::new()); }
    if config_hashes.fsb512 { addhash!(hashes, "fsb512", Fsb512::new()); }
    if config_hashes.sm3 { addhash!(hashes, "sm3", Sm3::new()); }
    if config_hashes.gost94_cryptopro { addhash!(hashes, "gost94_cryptopro", Gost94CryptoPro::new()); }
    if config_hashes.gost94_test { addhash!(hashes, "gost94_test", Gost94Test::new()); }
    if config_hashes.gost94_ua { addhash!(hashes, "gost94_ua", Gost94UA::new()); }
    if config_hashes.gost94_s2015 { addhash!(hashes, "gost94_s2015", Gost94s2015::new()); }
    if config_hashes.groestl224 { addhash!(hashes, "groestl224", Groestl224::new()); }
    if config_hashes.groestl256 { addhash!(hashes, "groestl256", Groestl256::new()); }
    if config_hashes.groestl384 { addhash!(hashes, "groestl384", Groestl384::new()); }
    if config_hashes.groestl512 { addhash!(hashes, "groestl512", Groestl512::new()); }
    if config_hashes.shabal192 { addhash!(hashes, "shabal192", Shabal192::new()); }
    if config_hashes.shabal224 { addhash!(hashes, "shabal224", Shabal224::new()); }
    if config_hashes.shabal256 { addhash!(hashes, "shabal256", Shabal256::new()); }
    if config_hashes.shabal384 { addhash!(hashes, "shabal384", Shabal384::new()); }
    if config_hashes.shabal512 { addhash!(hashes, "shabal512", Shabal512::new()); }

    hashes
}
