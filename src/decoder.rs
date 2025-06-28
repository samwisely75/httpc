use crate::utils::Result;
use bytes::Bytes;
use encoding_rs::SHIFT_JIS;
use flate2::read::DeflateDecoder;
use flate2::read::GzDecoder;
use std::io::Read;
use std::str;

pub const ENC_NONE: &str = ":plaintext:";
pub const ENC_GZIP: &str = "gzip";
pub const ENC_DEFLATE: &str = "deflate";
pub const ENC_ZSTD: &str = "zstd";

pub fn decode_gzip(data: &[u8]) -> Result<Bytes> {
    let mut decoder = GzDecoder::new(data);
    let mut decoded_data = Vec::new();
    decoder.read_to_end(&mut decoded_data)?;
    Ok(Bytes::copy_from_slice(&decoded_data))
    // Ok(str::from_utf8(&decoded_data)?.to_string())
}

pub fn decode_deflate(data: &[u8]) -> Result<Bytes> {
    let mut decoder = DeflateDecoder::new(data);
    let mut decoded_data = Vec::new();
    decoder.read_to_end(&mut decoded_data)?;
    Ok(Bytes::copy_from_slice(&decoded_data))
    // Ok(str::from_utf8(&decoded_data)?.to_string())
}

pub fn decode_zstd(data: &[u8]) -> Result<Bytes> {
    let decoded_data = zstd::decode_all(data)?;
    // Ok(str::from_utf8(&decoded_data)?.to_string())
    Ok(Bytes::copy_from_slice(&decoded_data))
}

pub fn decode_bytes(data: &[u8], encoding: &str) -> Result<String> {
    // Decompress the body bytes based on the encoding
    let body_bytes = match encoding {
        ENC_GZIP => decode_gzip(data),
        ENC_DEFLATE => decode_deflate(data),
        ENC_ZSTD => decode_zstd(data),
        _ => Ok(Bytes::copy_from_slice(data)),
    }?;

    // Try decoding the body as UTF-8 first, and if it fails,
    // fall back to SHIFT_JIS
    let body = match String::from_utf8(body_bytes.to_vec()) {
        Ok(s) => s,
        Err(utf8e) => {
            let (r, _, sjis_error) = SHIFT_JIS.decode(&body_bytes);
            if sjis_error {
                return Err(format!("Failed to decode body with utf8/shift-jis: {}", utf8e).into());
            }
            r.to_string()
        }
    };

    Ok(body)
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
        let s = str::from_utf8(&result).unwrap();
        assert_eq!(s, "test_123");
    }

    #[test]
    fn decode_deflate_should_return_correct_string() {
        let data = vec![43, 73, 45, 46, 137, 55, 52, 50, 6, 0];
        let result = decode_deflate(&data).unwrap();
        let s = str::from_utf8(&result).unwrap();
        assert_eq!(s, "test_123");
    }

    #[test]
    fn decode_zstd_should_return_correct_string() {
        let data = vec![
            40, 181, 47, 253, 0, 88, 65, 0, 0, 116, 101, 115, 116, 95, 49, 50, 51,
        ];
        let result = decode_zstd(&data).unwrap();
        let s = str::from_utf8(&result).unwrap();
        assert_eq!(s, "test_123");
    }

    #[test]
    fn test_decode_bytes_utf8() {
        let data = "Hello, 世界!".as_bytes();
        let result = decode_bytes(data, ENC_NONE).unwrap();
        assert_eq!(result, "Hello, 世界!");
    }

    #[test]
    fn test_decode_bytes_invalid_utf8_fallback() {
        // Invalid UTF-8 sequence
        let data = &[0xFF, 0xFE, 0x00, 0x48, 0x00, 0x65];
        let result = decode_bytes(data, ENC_NONE);
        assert!(result.is_ok()); // Should fall back to SHIFT_JIS or replacement
    }

    #[test]
    fn test_encoding_constants() {
        assert_eq!(ENC_NONE, ":plaintext:");
        assert_eq!(ENC_GZIP, "gzip");
        assert_eq!(ENC_DEFLATE, "deflate");
        assert_eq!(ENC_ZSTD, "zstd");
    }

    #[test]
    fn test_decode_empty_data() {
        let data = &[];
        let result = decode_bytes(data, ENC_NONE).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_decode_unknown_encoding() {
        let data = "test data".as_bytes();
        let result = decode_bytes(data, "unknown").unwrap();
        assert_eq!(result, "test data");
    }
}
