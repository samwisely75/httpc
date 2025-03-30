use crate::utils::Result;
use flate2::read::DeflateDecoder;
use flate2::read::GzDecoder;
use std::io::Read;
use std::str;
use zstd;

pub const ENC_NONE: &str = ":plaintext:";
pub const ENC_GZIP: &str = "gzip";
pub const ENC_DEFLATE: &str = "deflate";
pub const ENC_ZSTD: &str = "zstd";

pub fn decode_gzip(data: &[u8]) -> Result<String> {
    let mut decoder = GzDecoder::new(data);
    let mut decoded_data = Vec::new();
    decoder.read_to_end(&mut decoded_data)?;
    Ok(str::from_utf8(&decoded_data)?.to_string())
}

pub fn decode_deflate(data: &[u8]) -> Result<String> {
    let mut decoder = DeflateDecoder::new(data);
    let mut decoded_data = Vec::new();
    decoder.read_to_end(&mut decoded_data)?;
    Ok(str::from_utf8(&decoded_data)?.to_string())
}

pub fn decode_zstd(data: &[u8]) -> Result<String> {
    let decoded_data = zstd::decode_all(data)?;
    Ok(str::from_utf8(&decoded_data)?.to_string())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn decode_gzip_should_return_correct_string() {
        let data = vec![
            31, 139, 8, 0, 0, 0, 0, 0, 0, 255, 43, 73, 45, 46, 137, 55, 52, 50, 6, 0, 21, 191, 53,
            241, 8, 0, 0, 0,
        ];
        let result = decode_gzip(&data).unwrap();
        assert_eq!(result, "test_123");
    }

    #[test]
    fn decode_deflate_should_return_correct_string() {
        let data = vec![43, 73, 45, 46, 137, 55, 52, 50, 6, 0];
        let result = decode_deflate(&data).unwrap();
        assert_eq!(result, "test_123");
    }

    #[test]
    fn decode_zstd_should_return_correct_string() {
        let data = vec![
            40, 181, 47, 253, 0, 88, 65, 0, 0, 116, 101, 115, 116, 95, 49, 50, 51,
        ];
        let result = decode_zstd(&data).unwrap();
        assert_eq!(result, "test_123");
    }
}
