# sqlite

Example schema used in testing. The rows must be compatible (when in doubt match) with this, however the table name can
be changed in [config.toml](config.toml).

## Schema

```sql
PRAGMA journal_mode=WAL;

create table if not exists hashes (
    file_path text not null,
    file_size numeric,
    crc32 blob,
    md2 blob,
    md4 blob,
    md5 blob,
    sha1 blob,
    sha224 blob,
    sha256 blob,
    sha384 blob,
    sha512 blob,
    sha3_224 blob,
    sha3_256 blob,
    sha3_384 blob,
    sha3_512 blob,
    keccak224 blob,
    keccak256 blob,
    keccak384 blob,
    keccak512 blob,
    blake2s256 blob,
    blake2b512 blob,
    belt_hash blob,
    whirlpool blob,
    tiger blob,
    tiger2 blob,
    streebog256 blob,
    streebog512 blob,
    ripemd128 blob,
    ripemd160 blob,
    ripemd256 blob,
    ripemd320 blob,
    fsb160 blob,
    fsb224 blob,
    fsb256 blob,
    fsb384 blob,
    fsb512 blob,
    sm3 blob,
    gost94_cryptopro blob,
    gost94_test blob,
    gost94_ua blob,
    gost94_s2015 blob,
    groestl224 blob,
    groestl256 blob,
    groestl384 blob,
    groestl512 blob,
    shabal192 blob,
    shabal224 blob,
    shabal256 blob,
    shabal384 blob,
    shabal512 blob
);
```
