use std::fs;

use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use serde_derive::Deserialize;

use hasher::HashConfig;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// The path to hash the files inside
    #[arg(short = 'i', long, default_value_t = String::from("."))]
    pub input_path: String,

    #[clap(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    /// By default, things like IO and database errors will end execution when they happen
    #[arg(short = 'e', long, default_value_t = false)]
    pub continue_on_error: bool,

    /// Write hashes to the SQLite database in the config
    #[arg(short = 's', long, default_value_t = false)]
    pub sql_out: bool,

    /// Write hashes to stdout with JSON formatting
    #[arg(short = 'j', long, default_value_t = false)]
    pub json_out: bool,

    /// Pretty print JSON output
    #[arg(short = 'p', long, default_value_t = false)]
    pub pretty_json: bool,

    /// Enable WAL mode in the SQLite database while running
    #[arg(short = 'w', long, default_value_t = false)]
    pub use_wal: bool,

    /// The location of the config file
    #[arg(short = 'c', long, default_value_t = String::from("./config.toml"))]
    pub config_file: String,

    /// Reads file contents from stdin instead of any paths. --input-path becomes the path given in the output
    #[arg(short = 'n', long, default_value_t = false)]
    pub stdin: bool,

    /// Maximum number of subdirectories to descend when recursing directories
    #[arg(long, default_value_t = 20)]
    pub max_depth: usize,

    /// DON'T follow symlinks. Infinite loops are possible if this is off and there are bad symlinks.
    #[arg(long, default_value_t = false)]
    pub no_follow_symlinks: bool,

    /// Hash directories breadth first instead of depth first
    #[arg(short = 'b', long, default_value_t = false)]
    pub breadth_first: bool,

    /// Does not write hashes anywhere but stdout. Useful for benchmarking and if you hands are cold.
    #[arg(long, default_value_t = false)]
    pub dry_run: bool,
}

#[derive(Deserialize)]
pub struct Hashes {
    pub crc32: Option<bool>,
    pub md2: Option<bool>,
    pub md4: Option<bool>,
    pub md5: Option<bool>,
    pub sha1: Option<bool>,
    pub sha224: Option<bool>,
    pub sha256: Option<bool>,
    pub sha384: Option<bool>,
    pub sha512: Option<bool>,
    pub sha3_224: Option<bool>,
    pub sha3_256: Option<bool>,
    pub sha3_384: Option<bool>,
    pub sha3_512: Option<bool>,
    pub keccak224: Option<bool>,
    pub keccak256: Option<bool>,
    pub keccak384: Option<bool>,
    pub keccak512: Option<bool>,
    pub blake2s256: Option<bool>,
    pub blake2b512: Option<bool>,
    pub belt_hash: Option<bool>,
    pub whirlpool: Option<bool>,
    pub tiger: Option<bool>,
    pub tiger2: Option<bool>,
    pub streebog256: Option<bool>,
    pub streebog512: Option<bool>,
    pub ripemd128: Option<bool>,
    pub ripemd160: Option<bool>,
    pub ripemd256: Option<bool>,
    pub ripemd320: Option<bool>,
    pub fsb160: Option<bool>,
    pub fsb224: Option<bool>,
    pub fsb256: Option<bool>,
    pub fsb384: Option<bool>,
    pub fsb512: Option<bool>,
    pub sm3: Option<bool>,
    pub gost94_cryptopro: Option<bool>,
    pub gost94_test: Option<bool>,
    pub gost94_ua: Option<bool>,
    pub gost94_s2015: Option<bool>,
    pub groestl224: Option<bool>,
    pub groestl256: Option<bool>,
    pub groestl384: Option<bool>,
    pub groestl512: Option<bool>,
    pub shabal192: Option<bool>,
    pub shabal224: Option<bool>,
    pub shabal256: Option<bool>,
    pub shabal384: Option<bool>,
    pub shabal512: Option<bool>,
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

impl From<&Hashes> for HashConfig {
    fn from(hashes: &Hashes) -> Self {
        Self {
            crc32: hashes.crc32.unwrap_or(false),
            md2: hashes.md2.unwrap_or(false),
            md4: hashes.md4.unwrap_or(false),
            md5: hashes.md5.unwrap_or(false),
            sha1: hashes.sha1.unwrap_or(false),
            sha224: hashes.sha224.unwrap_or(false),
            sha256: hashes.sha256.unwrap_or(false),
            sha384: hashes.sha384.unwrap_or(false),
            sha512: hashes.sha512.unwrap_or(false),
            sha3_224: hashes.sha3_224.unwrap_or(false),
            sha3_256: hashes.sha3_256.unwrap_or(false),
            sha3_384: hashes.sha3_384.unwrap_or(false),
            sha3_512: hashes.sha3_512.unwrap_or(false),
            keccak224: hashes.keccak224.unwrap_or(false),
            keccak256: hashes.keccak256.unwrap_or(false),
            keccak384: hashes.keccak384.unwrap_or(false),
            keccak512: hashes.keccak512.unwrap_or(false),
            blake2s256: hashes.blake2s256.unwrap_or(false),
            blake2b512: hashes.blake2b512.unwrap_or(false),
            belt_hash: hashes.belt_hash.unwrap_or(false),
            whirlpool: hashes.whirlpool.unwrap_or(false),
            tiger: hashes.tiger.unwrap_or(false),
            tiger2: hashes.tiger2.unwrap_or(false),
            streebog256: hashes.streebog256.unwrap_or(false),
            streebog512: hashes.streebog512.unwrap_or(false),
            ripemd128: hashes.ripemd128.unwrap_or(false),
            ripemd160: hashes.ripemd160.unwrap_or(false),
            ripemd256: hashes.ripemd256.unwrap_or(false),
            ripemd320: hashes.ripemd320.unwrap_or(false),
            fsb160: hashes.fsb160.unwrap_or(false),
            fsb224: hashes.fsb224.unwrap_or(false),
            fsb256: hashes.fsb256.unwrap_or(false),
            fsb384: hashes.fsb384.unwrap_or(false),
            fsb512: hashes.fsb512.unwrap_or(false),
            sm3: hashes.sm3.unwrap_or(false),
            gost94_cryptopro: hashes.gost94_cryptopro.unwrap_or(false),
            gost94_test: hashes.gost94_test.unwrap_or(false),
            gost94_ua: hashes.gost94_ua.unwrap_or(false),
            gost94_s2015: hashes.gost94_s2015.unwrap_or(false),
            groestl224: hashes.groestl224.unwrap_or(false),
            groestl256: hashes.groestl256.unwrap_or(false),
            groestl384: hashes.groestl384.unwrap_or(false),
            groestl512: hashes.groestl512.unwrap_or(false),
            shabal192: hashes.shabal192.unwrap_or(false),
            shabal224: hashes.shabal224.unwrap_or(false),
            shabal256: hashes.shabal256.unwrap_or(false),
            shabal384: hashes.shabal384.unwrap_or(false),
            shabal512: hashes.shabal512.unwrap_or(false),
        }
    }
}
