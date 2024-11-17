# hasher

hasher is a program that can do multithreaded simultaneous hashing of files with up to 48 hashing algorithms while
only reading the file once. This means that in almost all cases the limiting factor in performance will be IO, and you
won't have to waste time reading the same data multiple times if you need multiple hashing algorithms.

## Commands

hasher has three main commands:

```shell
hash     # Hash the files in a directory
copy     # (not implemented) Copy files while hashing them
verify   # Verify files against stored hashes
download # (not implemented) Download and hash files
```

## Building

hasher requires a fairly modern version of Rust, preferably the latest stable release. Install it using the instructions
located [here](https://www.rust-lang.org/tools/install).

To build, run the following at the root of the repository:

```shell
cargo build -r
```

Go ahead and get yourself a drink while this is running, it will take a while. After this is complete your binary will
be located at `target/release/hasher` and can be moved wherever you desire, or leave it place and use `cargo run -r`.

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

### `hash`

```shell
Usage: hasher hash [OPTIONS] [SOURCE]

Arguments:
  [SOURCE]  Directory to hash

Options:
  -v, --verbose...                 Increase logging verbosity
  -q, --quiet...                   Decrease logging verbosity
  -e, --continue-on-error
  -s, --sql-out
  -j, --json-out
  -p, --pretty-json
  -w, --use-wal
  -c, --config-file <CONFIG_FILE>  [default: ./config.toml]
  -n, --stdin
      --max-depth <MAX_DEPTH>      [default: 20]
      --no-follow-symlinks
  -b, --breadth-first
      --dry-run
  -h, --help                       Print help
```

### `verify`

```shell
Verify files against stored hashes in the database

Usage: hasher verify [OPTIONS] <SOURCE>

Arguments:
  <SOURCE>  Directory to verify

Options:
  -m, --mismatches-only            Report only mismatches
  -v, --verbose...                 Increase logging verbosity
  -q, --quiet...                   Decrease logging verbosity
  -e, --continue-on-error
  -c, --config-file <CONFIG_FILE>  [default: ./config.toml]
      --max-depth <MAX_DEPTH>      [default: 20]
      --no-follow-symlinks
  -b, --breadth-first
      --dry-run
  -h, --help                       Print help
```

### Example Usage

Hash all files in the current directory and output hashes to stdout (JSON formatting) and store in SQLite:
```shell
hasher hash -j -s .
```

Verify files in a directory against stored hashes, showing only mismatches:
```shell
hasher verify -m /path/to/dir
```

### Config File

The config file (default `config.toml`) controls which hashes are calculated and database settings. See the example
[`config.toml`](config.toml) in the repository root for all options.

## Hashes

The following hashes are supported by the program:

- CRC32
- MD2
- MD4
- MD5
- SHA-1
- SHA-2
  - SHA-224 through SHA-512
- SHA-3
  - SHA3-224 through SHA3-512
- BLAKE2
  - Blake2s256, Blake2b512
- BelT
- Whirpool
- Tiger
- Streebog (GOST R 34.11-2012)
- RIPEMD
- FSB
- SM3
- GOST R 34.11-94
- Grøstl (Groestl)
- SHABAL

## Notes

### Memory Integrity

While hasher being written in Rust (aside from the sqlite driver) makes it immune to memory safety bugs, hasher does not
have any protections against any issues that may compromise memory integrity like cosmic bitflips. This is not a concern
for most activities on servers that have ECC memory, however this is important to keep in mind if you are doing very
large amounts of hashing on conventional computers with non-ECC memory. Trust, but verify.

### Other

Sponsored by 📼 🚙
