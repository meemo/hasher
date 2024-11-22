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
