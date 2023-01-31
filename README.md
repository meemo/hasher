# hasher

*I'm amazing at naming programs.*

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
$ ./hasher -h
A parallel file hashing program.

Usage: hasher [OPTIONS]

Options:
  -i, --input-path <INPUT_PATH>    The path to be hashed [default: .]
  -o, --output-path <OUTPUT_PATH>  The path to output hashes, {sha256}.json [default: ./hashes/]
  -c, --config-file <CONFIG_FILE>  The location of the config file [default: config.toml]
      --max-depth <MAX_DEPTH>      Maximum number of subdirectories to descend when recursing directories [default: 16]
      --crc32                      Whether to calculate a CRC32 hash [default: true]
      --md5                        MD5 hash [default: true]
      --sha1                       SHA-1 [default: true]
      --sha224                     SHA-224 [default: false]
      --sha256                     SHA-256 [default: false]
      --sha384                     SHA-384 [default: false]
      --sha512                     SHA-512 [default: false]
      --blake2b512                 Blake2b512 [default: false]
      --follow-symlinks            Whether or not to follow symlinks [default: true]
  -h, --help                       Print help
  -V, --version                    Print version
```

No arguments are required to be passed, however you most likely want to change the input path at least.

Note: Config files are not implemented.

### Logging

Logging is controlled through the `RUST_LOG` environment variable. Run the program like `RUST_LOG=info ./hasher` in
order to see the most information about what the program is doing.

## TODO

- Implement config files
- Add outputting hashes to SQL database (instead of JSON files)
- Optimize hashing, mainly in evening out file IO by reading another buffer while hashing (helps on spinning rust).
- Make logging controllable through args instead of just env variables

## Hashes

### Implemented

- CRC32
- MD5
- SHA-1
- SHA-2
  - SHA-224
  - SHA-256
  - SHA-384
  - SHA-512
- Blake2b512

### NOT Implemented

The following hashes are not implemented however then may be in the future:

- The rest of the BLAKE families
- SHA-3
- Whirpool
- Tiger
- Streebog (GOST R 34.11-2012)
- MD2
- MD4
- BelT
- SM3
- GOST R 34.11-94
- Grøstl (Groestl)
- FSB
- RIPEMD
- KangarooTwelve
- SHABAL
- Other CRC variants (these don't implement digest so they will be difficult to implement)
  - Adler CRC32 (aka Adler32)
    - In most cases this should be the same as CRC32, however it has the possibility of being different.
  - CRC16
  - CRC64
  - CRC128

## Notes

Sponsored by 📼 🚙
