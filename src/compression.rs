use flate2::{read::GzDecoder, write::GzEncoder, Compression};
use std::io::{self, Read, Write};

pub trait CompressionAlgorithm: Send + Sync {
    fn compress_file<R: Read, W: Write>(&self, reader: &mut R, writer: &mut W) -> io::Result<u64>;
    fn decompress_file<R: Read, W: Write>(&self, reader: &mut R, writer: &mut W)
        -> io::Result<u64>;
    fn extension(&self) -> &str;
    fn is_compressed_path(&self, path: &std::path::Path) -> bool {
        path.extension()
            .map_or(false, |ext| ext == self.extension().trim_start_matches('.'))
    }
}

pub struct GzipCompression {
    level: u32,
}

impl GzipCompression {
    pub fn new(level: u32) -> Self {
        Self {
            level: level.clamp(1, 9),
        }
    }
}

impl CompressionAlgorithm for GzipCompression {
    fn compress_file<R: Read, W: Write>(&self, reader: &mut R, writer: &mut W) -> io::Result<u64> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.level));
        let bytes_read = io::copy(reader, &mut encoder)?;
        writer.write_all(&encoder.finish()?)?;
        Ok(bytes_read)
    }

    fn decompress_file<R: Read, W: Write>(
        &self,
        reader: &mut R,
        writer: &mut W,
    ) -> io::Result<u64> {
        let mut decoder = GzDecoder::new(reader);
        io::copy(&mut decoder, writer)
    }

    fn extension(&self) -> &str {
        ".gz"
    }
}

pub fn get_compressor(algorithm: CompressionType, level: u32) -> GzipCompression {
    match algorithm {
        CompressionType::Gzip => GzipCompression::new(level),
    }
}

#[derive(Clone, Copy)]
pub enum CompressionType {
    Gzip,
}

pub fn compress_bytes(bytes: &[u8], algorithm: CompressionType, level: u32) -> io::Result<Vec<u8>> {
    let compressor = get_compressor(algorithm, level);
    let mut output = Vec::new();
    let mut reader = &bytes[..];
    compressor.compress_file(&mut reader, &mut output)?;
    Ok(output)
}

pub fn decompress_bytes(bytes: &[u8], algorithm: CompressionType) -> io::Result<Vec<u8>> {
    let compressor = get_compressor(algorithm, 6);
    let mut output = Vec::new();
    let mut reader = &bytes[..];
    compressor.decompress_file(&mut reader, &mut output)?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gzip_compression() {
        let data = b"test compression data".repeat(100);
        let compressed = compress_bytes(&data, CompressionType::Gzip, 6).unwrap();
        let decompressed = decompress_bytes(&compressed, CompressionType::Gzip).unwrap();
        assert_eq!(data.to_vec(), decompressed);
    }

    #[test]
    fn test_compression_levels() {
        let data = b"test compression data".repeat(100);
        let high_compressed = compress_bytes(&data, CompressionType::Gzip, 9).unwrap();
        let low_compressed = compress_bytes(&data, CompressionType::Gzip, 1).unwrap();

        // Higher compression level should generally produce smaller output
        assert!(high_compressed.len() <= low_compressed.len());
    }

    #[test]
    fn test_compression_edge_cases() {
        // Test empty input
        let empty = b"";
        let compressed = compress_bytes(empty, CompressionType::Gzip, 6).unwrap();
        let decompressed = decompress_bytes(&compressed, CompressionType::Gzip).unwrap();
        assert_eq!(empty.to_vec(), decompressed);

        // Test single byte
        let single = b"x";
        let compressed = compress_bytes(single, CompressionType::Gzip, 6).unwrap();
        let decompressed = decompress_bytes(&compressed, CompressionType::Gzip).unwrap();
        assert_eq!(single.to_vec(), decompressed);

        // Test repeating pattern (highly compressible)
        let repeating = b"abcdef".repeat(1000);
        let compressed = compress_bytes(&repeating, CompressionType::Gzip, 6).unwrap();
        assert!(compressed.len() < repeating.len());
        let decompressed = decompress_bytes(&compressed, CompressionType::Gzip).unwrap();
        assert_eq!(repeating.to_vec(), decompressed);
    }
}
