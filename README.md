# hasher

hasher is a program that can do multithreaded simultaneous hashing of files with up to 48 hashing algorithms while
only reading the file once. This means that in almost all cases the limiting factor in performance will be IO, and you
won't have to waste time reading the same data multiple times if you need multiple hashing algorithms.

## Building

hasher requires a fairly modern version of stable Rust. All development is done on the latest stable release, and
older versions are likely to cause issues.

Install Rust using the instructions located [here](https://www.rust-lang.org/tools/install).

To build, run the following at the root of the repository:

```shell
cargo build -r
```

After this is complete your binary will be located at `target/release/hasher` and can be moved wherever you desire,
or leave it in place and use `cargo run -r`.

### musl libc Builds

On Linux systems you may run into some glibc version issues if you, for example, build on an Arch Linux system and then
run on a Debian Stable system. The easiest way to alleviate this issue is to build the application on the target you
are running, however that's not always possible or desirable, so there is another option: static compilation with libc
built into the binary. This can be done by following the following steps:

```shell
sudo apt install musl-tools  # Or equivalent package for musl-gcc on your system

rustup target add x86_64-unknown-linux-musl
cargo build -r --target=x86_64-unknown-linux-musl
```

This will create a release build in the same location as normal builds but now it will not use glibc. Note that this
will slightly impact performance, as the program is being compiled for a generic x86_64 system, and can't include
instructions from extensions that only exist on modern CPUs.

## Usage

The basic structure for usage is:

```shell
hasher {command} {options} {directories}
```

With all commands supporting the following basic options (though some may do nothing):

- `-v`/`--verbose`, `-q`/`--quiet`
  - Control logging verbosity. Use `-vvv` for maximum output
- `-e`/`--fail-fast`
  - Stop after encountering an error (by default errors are not fatal)
- `-Q`/`--silent-failures`
  - Silence error messages and skip notifications (errors will still not be fatal unless --fail-fast is used)
- `-r`/`--retry-count`
  - Number of retries for operations (default: 3)
- `-d`/`--retry-delay`
  - Delay in seconds between retries (default: 5)
- `-s`/`--sql-only`
  - Only output to SQLite database
- `-j`/`--json-only`
  - Only output to JSON
- `-p`/`--pretty-json`
  - Pretty print JSON output
- `-c`/`--config-file`
  - Path to config file (default: ./config.toml)
- `-w`/`--use-wal`
  - Use sqlite Write Ahead Logging
- `-n`/`--stdin`
  - Hash data from stdin instead of files
- `-m`/`--max-depth`
  - Maximum number of directories to traverse (default: 30)
- `-L`/`--no-follow-symlinks`
  - Do not follow symlinks (useful in case there are loops)
- `-b`/`--breadth-first`
  - Hash all files in the top level directory first before lower level directories
- `-t`/`--dry-run`
  - Run without actually saving anything
- `-D`/`--db-path`
  - Override the database path from config
- `-z`/`--compress`
  - Compress files when writing to disk (for copy/download)
- `-C`/`--hash-compressed`
  - Hash the compressed file instead of uncompressed
- `-x`/`--decompress`
  - Decompress gzip compressed files before hashing
- `-B`/`--hash-both`
  - Hash both compressed and decompressed content for compressed files
- `--compression-level`
  - Compression level (1-9, default: 6)

### `hash`: Hash Files/Directories

```shell
hasher hash [OPTIONS] [SOURCE]
```

Hash files in a directory. If no source is provided, the current directory is used.

Special options:
- `-n`/`--stdin`
  - Hash data from stdin instead of files

### `verify`: Verify Hashes

```shell
hasher verify [OPTIONS]
```

Verify files against stored hashes in the database.

Special options:
- `-M`/`--mismatches-only`
  - Only output when files fail to verify

### `copy`: Copy Files

```shell
hasher copy [OPTIONS] <SOURCE> <DESTINATION>
```

Copy files while hashing them.

Special options:
- `-S`/`--store-source-path`
  - Store source path instead of destination path in database
- `-k`/`--skip-existing`
  - Skip copying files that already exist in the destination
- `-H`/`--no-hash-existing`
  - Skip hash comparison when checking existing files (only check if it exists/size)

### `download`: Download Files

```shell
hasher download [OPTIONS] <SOURCE> <DESTINATION>
```

Download and hash files. SOURCE can be either a URL or a file containing URLs (one per line).

Special options:
- `-N`/`--no-clobber`
  - Do not replace already downloaded files

### Config File

The config file (default `config.toml`) controls which hashes are calculated and database settings. See the example
[`config.toml`](config.toml) in the repository root for all options.

## Supported Hashes

The following hashes are supported:

- CRC32
- MD2, MD4, MD5
- SHA-1
- SHA-2 (SHA-224 through SHA-512)
- SHA-3 (SHA3-224 through SHA3-512)
- BLAKE2 (Blake2s256, Blake2b512)
- BelT
- Whirlpool
- Tiger/Tiger2
- Streebog (GOST R 34.11-2012)
- RIPEMD (128/160/256/320)
- FSB
- SM3
- GOST R 34.11-94
- Grøstl
- SHABAL

## Testing

There are some unit tests for the crucial bits of the which can be tested with:

```shell
cargo test
```

However the [`test.py`](test.py) script has been created to test that the major user-facing functions are working:

```shell
python3 test.py
```

The only requirements for this script are a reasonably modern version of Python 3.

## Code Style

hasher follows `rustfmt` code style, and it can be applied in a single command with:

```shell
rustfmt --edition 2021 src/*.rs src/commands/*.rs
```

## Notes

### Memory Integrity

While hasher being written in Rust makes it immune to memory safety bugs, hasher does not have any protections against
issues that may compromise memory integrity like cosmic bitflips. This is not a concern for most activities on servers
that have ECC memory, however this is important to keep in mind if you are doing very large amounts of hashing on
conventional computers with non-ECC memory. Trust, but verify.

### Changelog

See [`docs/CHANGELOG.md`](docs/CHANGELOG.md) for version history.

### Other

Sponsored by 📼 🚙
