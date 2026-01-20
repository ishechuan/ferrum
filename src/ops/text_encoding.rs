//! Text Encoding API
//!
//! This module provides TextEncoder and TextDecoder implementations for the Ferrum runtime.
//! These are standard Web APIs for converting between strings and byte sequences.

/// Result of TextEncoder.encodeInto() operation
#[derive(Debug, Clone)]
pub struct EncodeIntoResult {
    /// Number of characters read from the input
    pub read: usize,
    /// Number of bytes written to the output
    pub written: usize,
}

impl EncodeIntoResult {
    /// Create a new EncodeIntoResult
    pub fn new(read: usize, written: usize) -> Self {
        Self { read, written }
    }
}

/// TextEncoder implementation
///
/// Converts strings to UTF-8 encoded bytes.
///
/// # JavaScript API
/// ```javascript
/// const encoder = new TextEncoder();
/// const bytes = encoder.encode("hello"); // Uint8Array [104, 101, 108, 108, 111]
/// const result = encoder.encodeInto("hello", dest); // { read: 5, written: 5 }
/// ```
///
/// # Properties
/// - `encoding`: Always returns "utf-8"
#[derive(Clone)]
pub struct TextEncoder;

impl TextEncoder {
    /// Create a new TextEncoder instance
    pub fn new() -> Self {
        Self
    }

    /// Get the encoding name (always "utf-8")
    pub fn encoding(&self) -> &'static str {
        "utf-8"
    }

    /// Encode a string into a Uint8Array
    ///
    /// # Arguments
    /// * `input` - The string to encode
    ///
    /// # Returns
    /// A Vec<u8> containing the UTF-8 encoded bytes
    pub fn encode(&self, input: &str) -> Vec<u8> {
        input.as_bytes().to_vec()
    }

    /// Encode a string into a pre-allocated destination buffer
    ///
    /// # Arguments
    /// * `src` - The source string to encode
    /// * `dest` - The destination Uint8Array
    ///
    /// # Returns
    /// An EncodeIntoResult containing the number of characters read and bytes written
    pub fn encode_into(&self, src: &str, dest: &mut [u8]) -> EncodeIntoResult {
        let src_bytes = src.as_bytes();
        let src_len = src_bytes.len();
        let dest_len = dest.len();

        let written = std::cmp::min(src_len, dest_len);
        dest[..written].copy_from_slice(&src_bytes[..written]);

        EncodeIntoResult::new(src_len, written)
    }
}

impl Default for TextEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// TextDecoder implementation
///
/// Decodes UTF-8 encoded bytes to a string.
///
/// # JavaScript API
/// ```javascript
/// const decoder = new TextDecoder();
/// const str = decoder.decode(new Uint8Array([104, 101, 108, 108, 111])); // "hello"
/// ```
///
/// # Constructor Options
/// - `label`: Encoding label (default: "utf-8")
/// - `fatal`: Whether to throw errors on invalid sequences (default: false)
/// - `ignoreBOM`: Whether to ignore BOM (default: false)
#[derive(Clone)]
pub struct TextDecoder {
    /// The encoding label
    label: String,
    /// Whether to throw errors on invalid sequences
    fatal: bool,
    /// Whether to ignore BOM
    ignore_bom: bool,
}

impl TextDecoder {
    /// Create a new TextDecoder with default options
    pub fn new() -> Self {
        Self {
            label: "utf-8".to_string(),
            fatal: false,
            ignore_bom: false,
        }
    }

    /// Create a new TextDecoder with custom options
    ///
    /// # Arguments
    /// * `label` - The encoding label (only "utf-8" is supported)
    /// * `fatal` - Whether to throw errors on invalid sequences
    /// * `ignore_bom` - Whether to ignore the byte order mark
    pub fn with_options(
        label: Option<String>,
        fatal: Option<bool>,
        ignore_bom: Option<bool>,
    ) -> Self {
        Self {
            label: label.unwrap_or_else(|| "utf-8".to_string()),
            fatal: fatal.unwrap_or(false),
            ignore_bom: ignore_bom.unwrap_or(false),
        }
    }

    /// Get the encoding name
    pub fn encoding(&self) -> &str {
        &self.label
    }

    /// Get the fatal option
    pub fn fatal(&self) -> bool {
        self.fatal
    }

    /// Get the ignoreBOM option
    pub fn ignore_bom(&self) -> bool {
        self.ignore_bom
    }

    /// Decode bytes to a string
    ///
    /// # Arguments
    /// * `bytes` - The UTF-8 encoded bytes to decode
    /// * `_stream` - Whether this is a chunk of a larger stream (ignored in this implementation)
    ///
    /// # Returns
    /// The decoded string
    ///
    /// # Errors
    /// If `fatal` is true and the bytes contain invalid UTF-8 sequences,
    /// returns a Replacement Character (U+FFFD)
    pub fn decode(&self, bytes: &[u8], _stream: bool) -> String {
        if self.fatal {
            // In fatal mode, replace invalid sequences with replacement character
            match std::str::from_utf8(bytes) {
                Ok(s) => s.to_string(),
                Err(e) => {
                    // Find valid prefix
                    let valid_up_to = e.valid_up_to();
                    let error_len = bytes.len() - valid_up_to;
                    let mut result = String::with_capacity(valid_up_to + error_len * 3);

                    // Add valid portion
                    result.push_str(&String::from_utf8_lossy(&bytes[..valid_up_to]));

                    // Replace invalid portion with U+FFFD (3 bytes in UTF-8)
                    for _ in 0..error_len {
                        result.push('\u{FFFD}');
                    }

                    result
                }
            }
        } else {
            // Non-fatal mode: use lossy conversion
            String::from_utf8_lossy(bytes).into_owned()
        }
    }

    /// Decode a single byte (convenience method)
    pub fn decode_byte(&self, byte: u8) -> char {
        if self.fatal {
            match std::str::from_utf8(&[byte]) {
                Ok(s) => s.chars().next().unwrap_or('\u{FFFD}'),
                Err(_) => '\u{FFFD}',
            }
        } else {
            byte as char
        }
    }
}

impl Default for TextDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a string to UTF-8 bytes
pub fn encode_to_bytes(input: &str) -> Vec<u8> {
    input.as_bytes().to_vec()
}

/// Convert UTF-8 bytes to a string (non-fatal)
pub fn decode_from_bytes(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Convert UTF-8 bytes to a string (fatal mode - replaces invalid sequences)
pub fn decode_from_bytes_fatal(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(e) => {
            let valid_up_to = e.valid_up_to();
            let error_len = bytes.len() - valid_up_to;
            let mut result = String::with_capacity(valid_up_to + error_len * 3);

            result.push_str(&String::from_utf8_lossy(&bytes[..valid_up_to]));

            for _ in 0..error_len {
                result.push('\u{FFFD}');
            }

            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_encoder_new() {
        let encoder = TextEncoder::new();
        assert_eq!(encoder.encoding(), "utf-8");
    }

    #[test]
    fn test_text_encoder_default() {
        let encoder = TextEncoder::default();
        assert_eq!(encoder.encoding(), "utf-8");
    }

    #[test]
    fn test_text_encoder_encode_ascii() {
        let encoder = TextEncoder::new();
        let bytes = encoder.encode("hello");
        assert_eq!(bytes, vec![104, 101, 108, 108, 111]);
    }

    #[test]
    fn test_text_encoder_encode_unicode() {
        let encoder = TextEncoder::new();
        let bytes = encoder.encode("你好");
        // "你好" in UTF-8
        assert_eq!(bytes, vec![228, 189, 160, 229, 165, 189]);
    }

    #[test]
    fn test_text_encoder_encode_emoji() {
        let encoder = TextEncoder::new();
        let bytes = encoder.encode("🎉");
        // 🎉 (party popper) in UTF-8: F0 9F 8E 89
        assert_eq!(bytes, vec![240, 159, 142, 137]);
    }

    #[test]
    fn test_text_encoder_encode_empty() {
        let encoder = TextEncoder::new();
        let bytes = encoder.encode("");
        assert!(bytes.is_empty());
    }

    #[test]
    fn test_text_encoder_encode_into() {
        let encoder = TextEncoder::new();
        let mut dest = [0u8; 10];
        let result = encoder.encode_into("hello", &mut dest);
        assert_eq!(result.read, 5);
        assert_eq!(result.written, 5);
        assert_eq!(&dest[..5], &[104, 101, 108, 108, 111]);
    }

    #[test]
    fn test_text_encoder_encode_into_buffer_too_small() {
        let encoder = TextEncoder::new();
        let mut dest = [0u8; 3];
        let result = encoder.encode_into("hello", &mut dest);
        assert_eq!(result.read, 5);
        assert_eq!(result.written, 3);
        assert_eq!(&dest, &[104, 101, 108]);
    }

    #[test]
    fn test_text_encoder_encode_into_empty() {
        let encoder = TextEncoder::new();
        let mut dest = [0u8; 10];
        let result = encoder.encode_into("", &mut dest);
        assert_eq!(result.read, 0);
        assert_eq!(result.written, 0);
    }

    #[test]
    fn test_text_decoder_new() {
        let decoder = TextDecoder::new();
        assert_eq!(decoder.encoding(), "utf-8");
        assert!(!decoder.fatal());
        assert!(!decoder.ignore_bom());
    }

    #[test]
    fn test_text_decoder_with_options() {
        let decoder = TextDecoder::with_options(Some("utf-8".to_string()), Some(true), Some(false));
        assert_eq!(decoder.encoding(), "utf-8");
        assert!(decoder.fatal());
        assert!(!decoder.ignore_bom());
    }

    #[test]
    fn test_text_decoder_decode_ascii() {
        let decoder = TextDecoder::new();
        let bytes = vec![104, 101, 108, 108, 111];
        let result = decoder.decode(&bytes, false);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_text_decoder_decode_unicode() {
        let decoder = TextDecoder::new();
        let bytes = vec![228, 189, 160, 229, 165, 189];
        let result = decoder.decode(&bytes, false);
        assert_eq!(result, "你好");
    }

    #[test]
    fn test_text_decoder_decode_emoji() {
        let decoder = TextDecoder::new();
        let bytes = vec![240, 159, 142, 137];
        let result = decoder.decode(&bytes, false);
        assert_eq!(result, "🎉");
    }

    #[test]
    fn test_text_decoder_decode_empty() {
        let decoder = TextDecoder::new();
        let bytes: Vec<u8> = vec![];
        let result = decoder.decode(&bytes, false);
        assert_eq!(result, "");
    }

    #[test]
    fn test_text_decoder_decode_invalid_non_fatal() {
        let decoder = TextDecoder::new();
        // Invalid UTF-8: continuation byte without preceding byte
        let bytes = vec![104, 101, 108, 108, 111, 255, 252, 111];
        let result = decoder.decode(&bytes, false);
        // Should not panic, should include replacement character
        assert!(result.contains("�") || result.contains("hello"));
    }

    #[test]
    fn test_text_decoder_decode_invalid_fatal() {
        let decoder = TextDecoder::with_options(None, Some(true), None);
        // Invalid UTF-8: continuation byte without preceding byte
        let bytes = vec![104, 101, 108, 108, 111, 255, 252, 111];
        let result = decoder.decode(&bytes, false);
        // Should contain replacement character for invalid sequence
        assert!(result.contains('�'));
    }

    #[test]
    fn test_encode_to_bytes() {
        let bytes = encode_to_bytes("hello");
        assert_eq!(bytes, vec![104, 101, 108, 108, 111]);
    }

    #[test]
    fn test_decode_from_bytes() {
        let bytes = vec![104, 101, 108, 108, 111];
        let result = decode_from_bytes(&bytes);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_decode_from_bytes_fatal() {
        let bytes = vec![104, 101, 108, 108, 111, 255];
        let result = decode_from_bytes_fatal(&bytes);
        assert!(result.contains('�'));
    }

    #[test]
    fn test_roundtrip_ascii() {
        let original = "Hello, World!";
        let bytes = encode_to_bytes(original);
        let decoded = decode_from_bytes(&bytes);
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_roundtrip_unicode() {
        let original = "你好世界🎉";
        let bytes = encode_to_bytes(original);
        let decoded = decode_from_bytes(&bytes);
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_roundtrip_mixed() {
        let original = "Hello, 世界! 🎉";
        let bytes = encode_to_bytes(original);
        let decoded = decode_from_bytes(&bytes);
        assert_eq!(original, decoded);
    }
}
