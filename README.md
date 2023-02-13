# hasher

hasher is a program that will be able compute a number of different hashes while only reading a file once. A buffer is
read and then threads are spawned for each selected hash, resulting in huge speed gains over hashing sequentially.

Currently this will spit out a bunch of JSON files at the designated directory, however this will change soon.

## Building

hasher requires a fairly modern version of Rust, preferably the latest. Install it using the instructions located
[here](https://www.rust-lang.org/tools/install) for the latest stable release.

To build, run the following at the root of the repository:

```
cargo build -r
```

After this is complete your binary will be located at `target/release/hasher` and can be moved wherever you desire (or
not, I'm not your dad).

## Usage

### General

```
$ ./hasher --help
A parallel file hashing program.

Usage: hasher [OPTIONS]

Options:
  -i, --input-path <INPUT_PATH>
          The path to be hashed [default: .]
  -v, --verbose...
          More output per occurrence
  -q, --quiet...
          Less output per occurrence
  -j, --json-output-path <JSON_OUTPUT_PATH>
          The path to output hashes, {path}/{sha256}.json [default: ./hashes]
  -c, --config-file <CONFIG_FILE>
          The location of the config file [default: ./config.toml]
      --stdin
          Reads file contents from stdin instead of any paths. --input-path becomes the path given in the output. Note: input must be smaller than the avaliable RAM
      --max-depth <MAX_DEPTH>
          Maximum number of subdirectories to descend when recursing directories [default: 16]
      --skip-files <SKIP_FILES>
          Number of files (inclusive) to skip before beginning to hash a directory [default: 0]
      --no-follow-symlinks
          DON'T follow symlinks
      --breadth-first
          Hash directories breadth first instead of depth first
  -h, --help
          Print help
  -V, --version
          Print version
```

### Config File

In the root of the repository there is a file named `config.toml` and the values within should be modified to suit your
needs. Do not remove any lines or change e.g. booleans to strings, otherwise the program will not run.

The config file will by default be looked for at `./config.toml` when the executable is current (the current working
directory). If you wish to specify a different location for this then use the `--config-file <path>` option.

## TODO

- Add outputting hashes to SQL database (instead of JSON files).
  - --json-out and --sql-out args

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
