use std::iter::zip;

use vst3::Steinberg::{char16, char8};

pub fn copy_u128_to_char8(source: &u128, target: &mut [char8; 16]) {
    target.fill(0);

    let source_bytes = source.to_be_bytes();

    for (source_byte, target_byte) in zip(source_bytes, target) {
        *target_byte = source_byte as _;
    }
}

pub fn copy_str_to_char8(source: &str, target: &mut [char8]) {
    target.fill(0);

    let source_bytes = source.as_bytes();

    // Account for null terminator
    assert!(target.len() > source_bytes.len());

    for (source_byte, target_byte) in zip(source_bytes, target) {
        *target_byte = *source_byte as _;
    }
}

pub fn copy_str_to_char16(source: &str, target: &mut [char16]) {
    target.fill(0);

    let source_wide: Vec<_> = source.encode_utf16().collect();

    // Account for null terminator
    assert!(target.len() > source_wide.len());

    for (source_char, target_char) in zip(source_wide, target) {
        *target_char = source_char as _;
    }
}
