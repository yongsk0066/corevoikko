// VFST binary format: header parsing, validation
// Origin: Transducer.cpp:163-178

use crate::VfstError;

/// VFST header magic constants (little-endian).
const COOKIE1: u32 = 0x0001_3A6E;
const COOKIE2: u32 = 0x0003_51FA;

/// Size of the VFST binary header in bytes.
pub const HEADER_SIZE: usize = 16;

/// Parsed VFST file header.
///
/// The header occupies the first 16 bytes of a VFST binary file:
/// - bytes 0..4: cookie1 (magic number)
/// - bytes 4..8: cookie2 (magic number)
/// - byte 8: weighted flag (0x00 = unweighted, 0x01 = weighted)
/// - bytes 9..16: reserved (must be zero)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VfstHeader {
    /// Whether this is a weighted transducer.
    pub weighted: bool,
}

/// Parses and validates the 16-byte VFST binary header.
///
/// Returns the parsed header on success. Byte-swap detection is skipped because
/// WASM targets are always little-endian and dictionaries are stored in LE format.
///
/// Origin: Transducer::checkNeedForByteSwapping() -- Transducer.cpp:163-178
///         Transducer::isWeightedTransducerFile() -- Transducer.cpp:181-183
pub fn parse_header(data: &[u8]) -> Result<VfstHeader, VfstError> {
    if data.len() < HEADER_SIZE {
        return Err(VfstError::TooShort {
            expected: HEADER_SIZE,
            actual: data.len(),
        });
    }

    let cookie1 = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
    let cookie2 = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);

    if cookie1 != COOKIE1 || cookie2 != COOKIE2 {
        return Err(VfstError::InvalidMagic);
    }

    let weighted = data[8] == 0x01;

    Ok(VfstHeader { weighted })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_header(weighted: bool) -> Vec<u8> {
        let mut buf = vec![0u8; HEADER_SIZE];
        buf[..4].copy_from_slice(&COOKIE1.to_le_bytes());
        buf[4..8].copy_from_slice(&COOKIE2.to_le_bytes());
        buf[8] = if weighted { 0x01 } else { 0x00 };
        buf
    }

    #[test]
    fn parse_unweighted_header() {
        let data = make_header(false);
        let header = parse_header(&data).unwrap();
        assert!(!header.weighted);
    }

    #[test]
    fn parse_weighted_header() {
        let data = make_header(true);
        let header = parse_header(&data).unwrap();
        assert!(header.weighted);
    }

    #[test]
    fn reject_too_short() {
        let data = [0u8; 8];
        let err = parse_header(&data).unwrap_err();
        assert!(matches!(
            err,
            VfstError::TooShort {
                expected: 16,
                actual: 8
            }
        ));
    }

    #[test]
    fn reject_invalid_magic() {
        let mut data = make_header(false);
        data[0] = 0xFF; // corrupt cookie1
        let err = parse_header(&data).unwrap_err();
        assert!(matches!(err, VfstError::InvalidMagic));
    }

    #[test]
    fn reject_reversed_cookies() {
        // Reversed byte order cookies should not be accepted (no byte-swap support)
        let cookie1_rev: u32 = 0x6E3A_0100;
        let cookie2_rev: u32 = 0xFA51_0300;
        let mut data = vec![0u8; HEADER_SIZE];
        data[..4].copy_from_slice(&cookie1_rev.to_le_bytes());
        data[4..8].copy_from_slice(&cookie2_rev.to_le_bytes());
        let err = parse_header(&data).unwrap_err();
        assert!(matches!(err, VfstError::InvalidMagic));
    }

    #[test]
    fn header_with_trailing_data() {
        let mut data = make_header(false);
        data.extend_from_slice(&[0u8; 100]); // extra data after header is fine
        let header = parse_header(&data).unwrap();
        assert!(!header.weighted);
    }
}
