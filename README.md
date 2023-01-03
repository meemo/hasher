# hasher

*I'm amazing at naming programs.*

hasher is a program that will be able compute a number of different hashes while only reading a file once.

## Building

hasher requires Rust, preferably a semi-modern version. Install Rust using the instructions
[here](https://www.rust-lang.org/tools/install) and install the latest stable release. Unstable will still work in
theory, however I haven't tested it.

To build, run the following at the root of the repository:

    cargo build -r

After this is complete your binary will be located at `target/release/hasher` and can be moved wherever you desire.

## Usage

## Temporary

This is currently how the program works:

    hasher [path to file]

No config files are implemented at the moment.

## Planned

    hasher [--config-file={path}] {path with files to hash}

The path will be recursively searched and every file within will be hashed with all algorithms specified in the config
file.

Config files must follow the template given in [`config.toml.template`](config.toml.template). If no config file path
argument is specified then the program will look for `config.toml` in the current working directory and use that.

In the future everything in the config file will be able to be passed as args, however for now a config file is
required.

## Hashes

The following hashes are the highest priority due to how common they are:

- SHA-256
- SHA-1
- MD5
- CRC32

The following hashes will be supported eventually:

- BLAKE2
- Other CRC variants
  - Adler CRC32 (aka Adler32)
    - In most cases this should be the same as CRC32, however it has the possibility of being different.
  - CRC16
  - CRC64
  - CRC128
- The rest of the SHA-2 family:
  - SHA-224
  - SHA-384
  - SHA-512
- SHA-3
- Whirpool
- Tiger
- Streebog (GOST R 34.11-2012)
- MD2
- MD4

The following hashes *may* be supported eventually, not guaranteed:

- BelT
- SM3
- GOST R 34.11-94
- Grøstl (Groestl)
- FSB
- RIPEMD
- KangarooTwelve
- SHABAL

## Exit Codes

The program will have specific exit codes for general types errors so that stderr doesn't always have to be read. stderr
will have a more verbose description of what went wrong. This table will be expanded as the program is developed.

## Temporary

| Exit Code | Description |
|-----------|-------------|
| 0         | No errors, program ran successfully. |
| 1         | Invalid argument(s) or number of argument(s) passed. |

## Planned

| Exit Code | Description |
|-----------|-------------|
| 0         | No errors, program ran successfully. |
| 1         | Invalid argument(s) or number of argument(s) passed. |
| 2         | No config file or invalid config file provided. |
| 3         | Path does not exist or does not contain any files. |

All non-0 exit codes should be considered an error.

## Notes

Sponsored by 📼 🚙
