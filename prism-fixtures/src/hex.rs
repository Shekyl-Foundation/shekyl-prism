// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Hex, as every committed fixture writes its bytes.

/// The bytes a string of hex digits spells, in either case. `None` for
/// half a byte or for anything that is not a hex digit.
pub fn decode(text: &str) -> Option<Vec<u8>> {
    let nibble = |digit: u8| char::from(digit).to_digit(16);
    let byte = |pair: &[u8]| u8::try_from(nibble(pair[0])? << 4 | nibble(pair[1])?).ok();
    let pairs = text.as_bytes().chunks_exact(2);
    if !pairs.remainder().is_empty() {
        return None;
    }
    pairs.map(byte).collect()
}

#[cfg(test)]
mod tests {
    use super::decode;

    #[test]
    fn hex_is_read_in_either_case_and_only_in_whole_bytes() {
        assert_eq!(decode("00ff10aB"), Some(vec![0x00, 0xff, 0x10, 0xab]));
        assert_eq!(decode(""), Some(Vec::new()));
        assert_eq!(decode("abc"), None, "half a byte");
        assert_eq!(decode("0g"), None, "not a hex digit");
        assert_eq!(decode("+1"), None, "a sign is not a hex digit");
    }
}
