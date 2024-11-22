# Changelog/Version History

## 0.8.5: Implement compression, download changes

- Implement compression for `copy` and `download`, as well as support for decompression in `verify`
- `download` now uses rustls instead of the system's native TLS implementation
- Add `--no-clobber` for `download`
- Create this changelog, backfill changes

## 0.8.4: Implement downloading, use rustfmt

- `download` command implemented
- Ran `rustfmt` for a consistent code style, which caused many changes to the files

## 0.8.3: Rewrite verification, various improvements

- `verify` command was producing the wrong hashes because they weren't stored/accessed in a deterministic way

## 0.8.2: Implement copying

- `copy` command implemented

## 0.8.1: Implement verification

- `verify` command implemented

## 0.8.0: Massive usage change, prepping for new feature

- Big changes in preparation for future versions
- Implemented the backend for the new command interface

## 0.7.3: JSON output redone, skip files removed, cleanup

- JSON output changed
- `--skip-files` removed

## 0.7.1: Improve error handling, CLI option adjustments

- Properly handle errors that were previously not handled well

## 0.7.0: Library rewrite, dependency bumps

- Core functionality has been rewritten in library format that can be used outside this program

## 0.6.0: Cleanup, improvements, bump dependencies

- Cleanup
- Bump dependencies

## 0.5.1: SQL out no longer prevents JSON out

- `--sql-out` can now be used with `--json-out`

## 0.5.0: SQLite DB support

- SQLite databases finally supported

## 0.4.4: Final final DB prep

- Entire program is now async
- Bring in database driver, implement Error for it

## 0.4.3: Prep before DB integration

- Cleanup like macro usage
- Fix groestl384 being spelled wrong in config
- Created sqlite schema document

## 0.4.2: --stdin, --skip-files

- Add `--stdin` and `--skip-files` arguments

## 0.4.1: Optimize hashing, add verbosity to args

- Add `-vvv`, `-qqq` functionality to control output of logging
- Remove `--write-config-template`
- Major changes to core functionality

## 0.4.0: More hashes, config files, cleanup

- Add additional hashes
- Add config files

## 0.3.0: First functional version (JSON output)

- Add JSON output

## 0.2.2: Logging and general improvements

- General improvements
- `rustfmt` ran

## 0.2.1: Fix crc32 not being logged

- CRC32 now logged

## 0.2.0: Command line arguments implemented

- Add initial CLI args, currently controlling each hash and various other things

## 0.1.1: Fleshing out features

- Breaking out code into more files

## 0.1.0: Basic functionality implemented

- Begin simultaneous hashing implementation

## 0.0.1: Initial version

- Proofs of concept, nothing useful
