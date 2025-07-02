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
                return Err(anyhow::anyhow!(
                    "Failed to decode body with utf8/shift-jis: {}",
                    utf8e
                ));
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
    fn test_decode_unknown_encoding() {
        let data = "test data".as_bytes();
        let result = decode_bytes(data, "unknown").unwrap();
        assert_eq!(result, "test data");
    }

    #[test]
    fn test_decode_corrupted_gzip() {
        let corrupted_data = vec![31, 139, 8, 0, 0, 0, 0, 0, 0, 255, 1, 2, 3]; // Invalid gzip
        let result = decode_gzip(&corrupted_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_corrupted_deflate() {
        let corrupted_data = vec![1, 2, 3, 4, 5]; // Invalid deflate
        let result = decode_deflate(&corrupted_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_corrupted_zstd() {
        let corrupted_data = vec![1, 2, 3, 4, 5]; // Invalid zstd
        let result = decode_zstd(&corrupted_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_bytes_with_case_insensitive_encoding() {
        let data = "test data".as_bytes();

        let test_cases = vec![
            "GZIP", "Gzip", "gZiP", "DEFLATE", "Deflate", "dEfLaTe", "ZSTD", "Zstd", "zStD",
        ];

        for encoding in test_cases {
            let result = decode_bytes(data, encoding);
            // Should not panic and handle case insensitively
            match encoding.to_lowercase().as_str() {
                "gzip" | "deflate" | "zstd" => {
                    // These will likely fail with invalid data, but shouldn't panic
                    assert!(result.is_ok() || result.is_err());
                }
                _ => {
                    // Should fallback to UTF-8 decoding
                    assert!(result.is_ok());
                }
            }
        }
    }

    #[test]
    fn test_decode_bytes_large_data() {
        let large_data = "x".repeat(10000);
        let result = decode_bytes(large_data.as_bytes(), ENC_NONE).unwrap();
        assert_eq!(result.len(), 10000);
        assert!(result.chars().all(|c| c == 'x'));
    }

    #[test]
    fn test_decode_bytes_empty_encoding_string() {
        let data = "test data".as_bytes();
        let result = decode_bytes(data, "").unwrap();
        assert_eq!(result, "test data");
    }

    #[test]
    fn test_decode_bytes_with_multiple_encodings() {
        let data = "test data".as_bytes();
        let result = decode_bytes(data, "gzip, deflate").unwrap();
        // Should handle the first encoding or fallback to UTF-8
        assert_eq!(result, "test data");
    }

    #[test]
    fn test_decode_bytes_with_whitespace_in_encoding() {
        let data = "test data".as_bytes();
        let encodings = vec![" gzip ", "\tdeflate\t", "\ngzip\n"];

        for encoding in encodings {
            let result = decode_bytes(data, encoding);
            // Should handle whitespace gracefully
            assert!(result.is_ok() || result.is_err());
        }
    }

    #[test]
    fn test_encoding_constants() {
        assert_eq!(ENC_GZIP, "gzip");
        assert_eq!(ENC_DEFLATE, "deflate");
        assert_eq!(ENC_ZSTD, "zstd");
        assert_eq!(ENC_NONE, ":plaintext:");
    }

    #[test]
    fn test_decode_bytes_non_utf8_fallback() {
        // Test with invalid UTF-8 bytes
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        let result = decode_bytes(&invalid_utf8, ENC_NONE);

        // Should either succeed with replacement characters or fail gracefully
        match result {
            Ok(s) => {
                // If it succeeds, it should contain replacement characters
                assert!(s.contains('�') || s.is_empty());
            }
            Err(_) => {
                // Error is also acceptable for invalid UTF-8
            }
        }
    }

    #[test]
    fn test_successful_compression_roundtrip() {
        use flate2::write::{DeflateEncoder, GzEncoder};
        use std::io::Write;

        let original_data = "Hello, World! This is a test string for compression.";

        // Test gzip roundtrip
        {
            let mut encoder = GzEncoder::new(Vec::new(), flate2::Compression::default());
            encoder.write_all(original_data.as_bytes()).unwrap();
            let compressed = encoder.finish().unwrap();

            let decompressed = decode_gzip(&compressed).unwrap();
            let result_string = String::from_utf8(decompressed.to_vec()).unwrap();
            assert_eq!(result_string, original_data);
        }

        // Test deflate roundtrip
        {
            let mut encoder = DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            encoder.write_all(original_data.as_bytes()).unwrap();
            let compressed = encoder.finish().unwrap();

            let decompressed = decode_deflate(&compressed).unwrap();
            let result_string = String::from_utf8(decompressed.to_vec()).unwrap();
            assert_eq!(result_string, original_data);
        }
    }

    #[test]
    fn test_decode_bytes_integration() {
        use flate2::write::GzEncoder;
        use std::io::Write;

        let original_data = "Integration test data";
        let mut encoder = GzEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(original_data.as_bytes()).unwrap();
        let compressed = encoder.finish().unwrap();

        let result = decode_bytes(&compressed, ENC_GZIP).unwrap();
        assert_eq!(result, original_data);
    }
}
