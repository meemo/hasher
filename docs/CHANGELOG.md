# Changelog/Version History

## 0.8.18: Code quality improvements and bug fixes

- Fixed major performance issue in verify command where files were being hashed twice unnecessarily
- Removed duplicate database insertion code in output.rs, now using the centralized insert_single_hash function
- Improved error handling in database initialization by replacing panic-prone expect() calls with proper error propagation
- Applied rustfmt formatting to entire codebase for consistent code style
- Enhanced maintainability by reducing code duplication and improving function organization
- These improvements ensure better performance and reliability when processing large datasets

## 0.8.17: Bug fixes: critical copy and decompression fixes

- Fixed critical bug where "hash match" skip was incorrectly triggered for non-existent destination files
- Improved file existence checking logic in copy command with clearer separation of conditions
- Added redundant safety checks to prevent attempting to read non-existent files
- Fixed double-buffering performance issue in direct file copy operations
- Enhanced error handling for edge cases where files might disappear during processing
- Fixed `--decompress` option to properly remove the .gz extension from output files
- Modified destination path handling to correctly strip .gz extension when decompressing

## 0.8.16: Confirguration enhancements, hash uncompressed option

- Added `--hash-uncompressed` (`-U`) option to always hash the uncompressed content even when source is compressed
- This is particularly useful when copying compressed files and wanting to store the hash of the original uncompressed content
- Implemented consistently across all commands (hash, verify, copy, download)
- Fixed handling of multiple compression-related flags to ensure consistent behavior
- Added support for specifying command line options as defaults in `config.toml`
- Added `[options]` section to config file for setting command line defaults
- Improved documentation around compression options and flag precedence
- Added fallback for missing config file with sensible defaults
- Enhanced documentation for database behavior and Windows path handling
- Clarified that command line options always override config file settings

## 0.8.15: Download output improvements

- Modified download command to output a single JSON object per download that combines both download status and hash information
- Fixed error handling across multiple commands to be more consistent
- Updated error type to be cloneable and handle errors more uniformly

## 0.8.14: Bug fixes: File extension and windows path handling

- Fixed a bug where `copy`ing compressed files would incorrectly duplicate extensions (e.g., `.tar.gz` becoming `.tar.gz.gz`)
- Fixed Windows paths having a `\\\\?\\` prefix

## 0.8.13: Error handling and output improvements

- Errors no longer fatal by default (Removed `--continue-on-error`)
- Added `--fail-fast` to make errors fatal
- Unified output control under `--silent-failures` (replaces `--skip-failures` and `--silent-skip`)
- Error messages and skip notifications shown by default
- Added short options for CLI arguments
- Updated test file to reflect new changes

## 0.8.12: Bug fix: Arguments properly implemented

- There were many issues with arguments not behaving properly now fixed:
  - Fixed stdin handling to properly respect json_only and sql_only flags
  - Fixed hash_compressed option handling in all commands
  - Made compression handling consistent across hash, verify, and copy commands
  - Fixed decompression handling in copy command
  - Fixed compression-related argument handling in file existence checks
  - Fixed verify command to properly give compressed hashes

## 0.8.11: Bug fixes, copy improvements, general changes

- `--hash-both` is now properly implemented across all commands
- Console output added for skipped files by default
- `--silent-skip` added to silence that console output
- General code cleanup
- rustfmt run
- Bump dependencies

## 0.8.10: Bug fix: Consistent database initialization

- Only `hash` would initialize databases that don't exist, now all commands do

## 0.8.9: Copy improvements, DB override option

- Add `--db-path` option to override the database path in the config file
- Add `--skip-existing` and `--no-hash-existing` options for `copy` to not copy files that already exist in the target
directory
- Update README to contain all available CLI options
- Minor code cleanup

## 0.8.8: Download improvements

- Download changes:
  - Downloading from a URL list file will no longer wait until the list is downloaded to display json
  - JSON output improved
  - Handle errors relating to downloading from URL list files
- Minor code cleanup

## 0.8.7: Download improvements, more tests

- `--skip-failures`, etc implemented for `download`
- Add compressed file contents hashing support to `hash`
- `download` now properly download from a list
- Make Cargo.toml description up-to-date
- Rewrote usage of README, added other sections

## 0.8.6: Add tests, fix bugs

- Implement testing
- Fixed a bug with `--store-source-path` using the short `-s` which is already in use
- Fixed a bug with `download` not creating the directory structures it should have been
- Clean up lib.rs a bit
- Remove excess macros

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
