# hasher

hasher is a program that can do multithreaded simultaneous hashing of files with up to 48 hashing algorithms while
only reading the file once. This means that in almost all cases the limiting factor in performance will be IO, and you
won't have to waste time reading the same data multiple times if you need multiple hashing algorithms.

Hashes are able to be output in 3 locations: stdout (with `-v` or higher), SQLite (with `--sql-out`), and JSON
(`--json-out`).

## Building

hasher requires a fairly modern version of Rust, preferably the latest stable release. Install it using the instructions
located [here](https://www.rust-lang.org/tools/install). Releases are currently not provided because of the reliance on
the config file. I plan to make this not an issue in a future release.

To build, run the following at the root of the repository:

```shell
cargo build -r
```

Go ahead and get yourself a drink while this is running, it will take a while. After this is complete your binary will
be located at `target/release/hasher` and can be moved wherever you desire, or leave it place and use `cargo run -r`.

### musl libc Builds

On Linux systems you may run into some glibc version issues if you, for example, build on an Arch Linux system and then
run on a Debian Stable system.. The easiest way to alleviate this issue is to build the application on the target you
are running, however that's not always possible or desirable so there is another option: static compilation with libc
built into the binary. This can be done with the following steps:

```shell
sudo apt install musl-tools  # Or equivalent package for musl-gcc on your system

rustup target add x86_64-unknown-linux-musl
cargo build -r --target=x86_64-unknown-linux-musl
```

This will create a release build in the same location as normal builds but now it will not use glibc.

## Config

hasher relies on config files to direct its operation. [`config.toml`](config.toml) is an example of a valid config
file, and is what it will look for unless another config path is specified with `-c`.

The database section is currently required, even if you only use json out. Those config entries will be used unless
--sql-out is specified.

The hashes section lists every single possible hash that can be calculated, with crc32, md5, sha1, and sha256 enabled by
default. You can remove lines of hashes to shorten the list, if the hash isn't in the list it will be disabled. If you
are using `--json-out` then sha256 is required, but otherwise you can pick and choose the hashes you want as long as you
have at least 1.

## Usage

### General

```shell
$ ./hasher --help
Multithreaded parallel hashing utility

Usage: hasher [OPTIONS]

Options:
  -i, --input-path <INPUT_PATH>    The path to hash the files inside [default: .]
  -v, --verbose...                 Increase logging verbosity
  -q, --quiet...                   Decrease logging verbosity
  -e, --continue-on-error          By default, things like IO and database errors will end execution when they happen
  -s, --sql-out                    Write hashes to the SQLite database in the config
  -j, --json-out                   Write hashes to stdout with JSON formatting
  -p, --pretty-json                Pretty print JSON output
  -w, --use-wal                    Enable WAL mode in the SQLite database while running
  -c, --config-file <CONFIG_FILE>  The location of the config file [default: ./config.toml]
  -n, --stdin                      Reads file contents from stdin instead of any paths. --input-path becomes the path given in the output
      --max-depth <MAX_DEPTH>      Maximum number of subdirectories to descend when recursing directories [default: 20]
      --no-follow-symlinks         DON'T follow symlinks. Infinite loops are possible if this is off and there are bad symlinks
  -b, --breadth-first              Hash directories breadth first instead of depth first
      --dry-run                    Does not write hashes anywhere but stdout. Useful for benchmarking and if you hands are cold
  -h, --help                       Print help
  -V, --version                    Print version
```

### Example Usage

Say I want to to hash all of the files in the `dev/` and view the output in stdout while writing to a SQLite database,
while accelerating the performance with WAL (write ahead log). Run this in the root of the repository after building:

```shell
./hasher -v --sql-out --use-wal -i dev/
```

Assuming the default `config.toml` is in the current working directory, this will hash everything to the database
`myhashes.db` in the current working directory.

### Config File

In the root of the repository there is a file named `config.toml` and the values within should be modified to suit your
needs. Do not remove any lines or change e.g. booleans to strings, otherwise the program will not run.

The config file will by default be looked for at `./config.toml` when the executable is current (the current working
directory). If you wish to specify a different location for this then use the `--config-file <path>` option.

### SQLite Database (`--sql-out`)

The database will automatically be created with the appropriate table name (by default `hashes`) regardless if the file
exists already.

For more information on the schema of this database, see [`sqlite.md`](sqlite.md).

### JSON Out (`--json-out`)

This option spits out the hashed files into the directory given. This is very much not recommended because the file
count of larger hashing runs will waste approximately 3x the size of the json itself due to sector size loss, so if you
are doing a bulk run it is highly suggested to use the database instead.

Each file is named with the sha256 hash of the file, so don't disable that hash. The contents of the JSON files are very
simple, and are the same names as the SQLite database's schema.

## Hashes

### Implemented

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

### Skipped

The following hashes were not implemented.

XOF hashes (no static output size so they require special handling to use):

- SHA-3
  - SHAKE128/SHAKE256
- BLAKE3
- KangarooTwelve

Hashes that don't implement the `digest` traits:

- The rest of the CRC variants
  - Adler CRC32 (aka Adler32)
    - In most cases this should be the same as CRC32, however it has the possibility of being different.
  - CRC16
  - CRC64
  - CRC128

## Notes

Sponsored by 📼 🚙
