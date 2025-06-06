# TODO

## Higher Priority

- `hash` changes
  - Add a lockfile for directories currently being hashed
    - Ensuring 2 instances of hasher don't hash the same major directory, don't worry about lower directories
    - Lockfile will contain the required information to resume an interrupted run of hashing from the dir
    - Make sure there's both *nix-like and windows support
- `download` changes
  - Add user agent to config file, use a different default
- Config changes
  - Make it so no config.toml is required to use all default settings (the repo's config.toml)
- Clean up the code

## Lower Priority

- EVENTUALLY add XOFs
  - Difficult because no simple digest interface thing that allows pasting in of functions like before
- Create actual documentation, rustdocs and expand companion md files
- Add CI integration for tests and codestyle
- Add postgresql support?

## Brainstorming

### Performance Improvements for Large-Scale Operations

#### Parallel Processing Enhancements
- Implement multi-threaded file processing using rayon or similar
- Create worker pool with configurable thread count
- Use bounded channels for backpressure control
- Separate file discovery from processing tasks
- Process files in batches to reduce overhead

#### Database Optimizations
- Implement connection pooling (critical bottleneck fix)
- Add bulk insert operations for batch processing
- Optimize SQLite pragmas for write-heavy workloads:
  - Increase cache size (PRAGMA cache_size = -2000000)
  - Use NORMAL synchronous mode for better performance
  - Enable memory-mapped I/O for large databases
  - Keep temp tables in memory
- Consider prepared statements for repeated operations
- Add transaction batching (e.g., 1000 inserts per transaction)

#### Memory-Efficient Processing
- Implement streaming hash computation for large files
- Use memory-mapped files for very large files (>100MB)
- Add configurable buffer sizes for file reading
- Implement file size thresholds for different processing strategies
- Consider using async I/O with tokio::io::BufReader

#### Progress Tracking and Resumability
- Implement checkpoint system to save progress
- Store processed file list for resume capability
- Add periodic checkpoint saves (e.g., every 5 minutes)
- Create progress reporting with ETA calculations
- Add real-time statistics (files/sec, MB/sec)

#### File System Optimizations
- Use OS-specific optimizations (e.g., d_type on Linux)
- Implement parallel directory traversal
- Add file filtering early in the pipeline:
  - Skip hidden files option
  - File size limits (min/max)
  - Extension blacklist/whitelist
  - Modification time filters
- Consider using FTS (File Tree Scan) APIs where available

#### Configuration Enhancements
```toml
[performance]
worker_threads = 0  # 0 = auto-detect
batch_size = 1000
max_memory_cache = 2048  # MB
use_mmap = true
mmap_threshold = 104857600  # 100MB
db_pool_size = 10
checkpoint_interval = 300  # seconds

[optimizations]
skip_hidden = true
skip_files_larger_than = 5368709120  # 5GB
skip_files_smaller_than = 0
skip_extensions = [".tmp", ".temp", ".lock"]
```

#### Error Handling Improvements
- Implement retry logic with exponential backoff
- Add per-file error recovery (don't stop entire operation)
- Create error summary report
- Add option to save failed files list for reprocessing
- Implement circuit breaker for repeated failures

#### Monitoring and Observability
- Add performance metrics collection
- Create live progress dashboard
- Implement logging levels for different verbosity
- Add memory usage monitoring
- Create performance profiling mode

#### Architecture Improvements
- Consider message queue for file processing pipeline
- Add support for distributed processing
- Implement job scheduling for large operations
- Create modular processing pipeline
- Add plugin system for custom processors
