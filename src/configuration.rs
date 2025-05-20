use std::fs;
use std::path::{Path, PathBuf};

use clap::Parser;
use clap_verbosity_flag::{Verbosity, WarnLevel};
use serde_derive::Deserialize;

use crate::utils::Error;
use hasher::HashConfig;

#[derive(Parser, Debug)]
#[command(author, version, about)]
pub struct HasherCli {
    #[command(subcommand)]
    pub command: HasherCommand,
}

#[derive(Parser, Debug)]
pub enum HasherCommand {
    /// Hash files in a directory
    Hash(HasherHashArgs),
    /// Copy files while hashing them
    Copy(HasherCopyArgs),
    /// Verify files against stored hashes in the database
    Verify(HasherVerifyArgs),
    /// Download and hash file at the given URL
    Download(HasherDownloadArgs),
}

#[derive(Parser, Debug, Clone)]
pub struct HasherOptions {
    #[clap(flatten)]
    pub verbose: Verbosity<WarnLevel>,

    /// Stop after encountering an error (by default errors are not fatal)
    #[arg(short = 'e', long)]
    pub fail_fast: bool,

    /// Silence error messages (errors will still not be fatal unless --fail-fast is used)
    #[arg(short = 'Q', long)]
    pub silent_failures: bool,

    /// Number of retries for operations (downloads, etc)
    #[arg(short = 'r', long, default_value_t = 3)]
    pub retry_count: u32,

    /// Delay in seconds between retries
    #[arg(short = 'd', long, default_value_t = 5)]
    pub retry_delay: u32,

    /// Only output to SQLite database (default: output to both SQLite and JSON)
    #[arg(short = 's', long)]
    pub sql_only: bool,

    /// Only output to JSON (default: output to both SQLite and JSON)
    #[arg(short = 'j', long)]
    pub json_only: bool,

    /// Pretty print the json output
    #[arg(short = 'p', long)]
    pub pretty_json: bool,

    /// Use sqlite Write Ahead Logging
    #[arg(short = 'w', long)]
    pub use_wal: bool,

    /// The path to the config file
    #[arg(short = 'c', long, default_value = "./config.toml")]
    pub config_file: PathBuf,

    /// Hash the "file" that's fed into stdin. The hashing path will become the "file name"
    #[arg(short = 'n', long)]
    pub stdin: bool,

    /// Maximum number of directories to traverse
    #[arg(short = 'm', long, default_value_t = 30)]
    pub max_depth: usize,

    /// Do not follow symlinks (useful in case there are loops)
    #[arg(short = 'L', long)]
    pub no_follow_symlinks: bool,

    /// Hash all files in the top level directory first before lower level directories
    #[arg(short = 'b', long)]
    pub breadth_first: bool,

    /// Run without actually saving anything
    #[arg(short = 't', long)]
    pub dry_run: bool,

    /// Override the database path from config
    #[arg(short = 'D', long)]
    pub db_path: Option<PathBuf>,

    /// Compress destination files with gzip
    #[arg(short = 'z', long)]
    pub compress: bool,

    /// Compression level (1-9 for gzip)
    #[arg(long, default_value_t = 6)]
    #[arg(value_parser = clap::value_parser!(u32).range(1..=9))]
    pub compression_level: u32,

    /// Hash the compressed file instead of uncompressed
    #[arg(short = 'C', long)]
    pub hash_compressed: bool,

    /// Decompress gzipped files before hashing
    #[arg(short = 'x', long)]
    pub decompress: bool,

    /// Hash both compressed and decompressed content for gzipped files
    #[arg(short = 'B', long)]
    pub hash_both: bool,

    /// Always hash the uncompressed content even when source is compressed
    #[arg(short = 'U', long)]
    pub hash_uncompressed: bool,
}

#[derive(Parser, Debug)]
pub struct HasherHashArgs {
    /// Directory to hash
    pub source: Option<PathBuf>,

    #[clap(flatten)]
    pub hash_options: HasherOptions,
}

#[derive(Parser, Debug)]
pub struct HasherVerifyArgs {
    /// Only output when files fail to verify instead of outputting every file
    #[arg(short = 'M', long)]
    pub mismatches_only: bool,

    #[clap(flatten)]
    pub hash_options: HasherOptions,
}

#[derive(Parser, Debug)]
pub struct HasherCopyArgs {
    /// Source directory
    pub source: PathBuf,
    /// Destination directory
    pub destination: PathBuf,

    /// Store source path instead of destination path in database
    #[arg(short = 'S', long)]
    pub store_source_path: bool,

    /// Skip copying files that already exist in the destination
    #[arg(short = 'k', long)]
    pub skip_existing: bool,

    /// Skip hash comparison when checking existing files (only check if it exists/size)
    #[arg(short = 'H', long)]
    pub no_hash_existing: bool,

    #[clap(flatten)]
    pub hash_options: HasherOptions,
}

#[derive(Parser, Debug)]
pub struct HasherDownloadArgs {
    /// Source URL or path to file with URLs
    pub source: PathBuf,
    /// Destination directory
    pub destination: PathBuf,

    /// Do not replace already downloaded files
    #[arg(short = 'N', long)]
    pub no_clobber: bool,

    #[clap(flatten)]
    pub hash_options: HasherOptions,
}

#[derive(Deserialize, Default)]
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

#[derive(Deserialize, Default)]
pub struct Options {
    pub fail_fast: Option<bool>,
    pub silent_failures: Option<bool>,
    pub retry_count: Option<u32>,
    pub retry_delay: Option<u32>,
    pub sql_only: Option<bool>,
    pub json_only: Option<bool>,
    pub pretty_json: Option<bool>,
    pub use_wal: Option<bool>,
    pub stdin: Option<bool>,
    pub max_depth: Option<usize>,
    pub no_follow_symlinks: Option<bool>,
    pub breadth_first: Option<bool>,
    pub dry_run: Option<bool>,
    pub compress: Option<bool>,
    pub compression_level: Option<u32>,
    pub hash_compressed: Option<bool>,
    pub decompress: Option<bool>,
    pub hash_both: Option<bool>,
    pub hash_uncompressed: Option<bool>,
}

#[derive(Deserialize)]
pub struct Config {
    pub database: Database,
    pub hashes: Hashes,
    pub options: Option<Options>,
}

pub fn get_config(path: &Path, db_path_override: Option<&Path>) -> Result<Config, Error> {
    // Try to read the config file, but use defaults if not found
    let config_str = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                // Use default config if file not found
                return Ok(Config {
                    database: Database {
                        db_string: "sqlite://myhashes.db".to_string(),
                        table_name: "hashes".to_string(),
                    },
                    hashes: Hashes {
                        crc32: Some(true),
                        md5: Some(true),
                        sha1: Some(true),
                        sha256: Some(true),
                        // All other hashes default to false
                        ..Default::default()
                    },
                    options: Some(Options::default()),
                });
            } else {
                return Err(Error::Config(format!("Failed to read config file: {}", e)));
            }
        }
    };

    let mut config: Config = toml::from_str(&config_str)
        .map_err(|e| Error::Config(format!("Invalid config file format: {}", e)))?;

    // Command line options always override config file settings
    if let Some(db_path) = db_path_override {
        config.database.db_string = format!("sqlite://{}", db_path.display());
    }

    Ok(config)
}

// Apply config file options to command-line options, respecting command-line precedence
pub fn apply_config_defaults(options: &mut HasherOptions, config: &Config) {
    // If no options section in config, nothing to do
    let Some(cfg_opts) = &config.options else { return };

    // Helper macro to apply boolean options (only set true if not already set)
    macro_rules! apply_bool_option {
        ($option:ident) => {
            if let Some(true) = cfg_opts.$option {
                if !options.$option {
                    options.$option = true;
                }
            }
        };
    }

    // Helper macro to apply numeric options (only set if command line has default value)
    macro_rules! apply_numeric_option {
        ($option:ident, $default:expr) => {
            if let Some(value) = cfg_opts.$option {
                if options.$option == $default {
                    options.$option = value;
                }
            }
        };
    }

    // Apply boolean options
    apply_bool_option!(fail_fast);
    apply_bool_option!(silent_failures);
    apply_bool_option!(sql_only);
    apply_bool_option!(json_only);
    apply_bool_option!(pretty_json);
    apply_bool_option!(use_wal);
    apply_bool_option!(stdin);
    apply_bool_option!(no_follow_symlinks);
    apply_bool_option!(breadth_first);
    apply_bool_option!(dry_run);
    apply_bool_option!(compress);
    apply_bool_option!(hash_compressed);
    apply_bool_option!(decompress);
    apply_bool_option!(hash_both);
    apply_bool_option!(hash_uncompressed);

    // Apply numeric options with their default values
    apply_numeric_option!(retry_count, 3);
    apply_numeric_option!(retry_delay, 5);
    apply_numeric_option!(max_depth, 30);

    // Special handling for compression_level to ensure it's clamped
    if let Some(level) = cfg_opts.compression_level {
        if options.compression_level == 6 { // default value
            options.compression_level = level.clamp(1, 9);
        }
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
