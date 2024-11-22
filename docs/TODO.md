# TODO

## Higher Priority

- Downloading changes
  - Add `--skip-failures` to skip e.g. 404s in lists of URLs without failure
    - Failures should have a specific JSON out detailing why they failed
  - Add user agent to config file, use a different default
- Config changes
  - Make it so no config.toml is required to use all default settings (the repo's config.toml)
- Add a lockfile for directories currently being hashed
  - Ensuring 2 instances of hasher don't hash the same major directory, don't worry about lower directories
  - Lockfile will contain the required information to resume an interrupted run of hashing from the dir
  - Make sure there's both linux and windows support
- Clean up the code, mainly to minimize the number of times the long list of hashes has to be done

## Lower Priority

- EVENTUALLY add XOFs
  - Difficult because no simple digest interface thing that allows pasting in of functions like before
- Create actual documentation, rustdocs and expand companion md files
- Add tests
- Add CI integration for tests and codestyle
- Add postgresql support?
