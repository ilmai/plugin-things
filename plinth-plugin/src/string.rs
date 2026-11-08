use std::{iter::zip, ops::RangeInclusive};

use vst3::Steinberg::{char16, char8};
use widestring::U16CStr;

const HIGH_SURROGATES: RangeInclusive<char16> = 0xD800..=0xDBFF;

pub fn char16_to_string(source: &[char16]) -> Option<String> {
    let Ok(cstr) = U16CStr::from_slice_truncate(source) else {
        return None;
    };

    cstr.to_string().ok()
}

pub fn copy_u128_to_char8(source: &u128, target: &mut [char8; 16]) {
    target.fill(0);

    let source_bytes = source.to_be_bytes();

    for (source_byte, target_byte) in zip(source_bytes, target) {
        *target_byte = source_byte as _;
    }
}

pub fn copy_str_to_char8(source: &str, target: &mut [char8]) {
    target.fill(0);

    // Account for null terminator, without splitting multi-byte chars
    let source_bytes = &source.as_bytes()[..source.floor_char_boundary(target.len().saturating_sub(1))];

    for (source_byte, target_byte) in zip(source_bytes, target) {
        *target_byte = *source_byte as _;
    }
}

pub fn copy_str_to_char16(source: &str, target: &mut [char16]) {
    target.fill(0);

    // Account for null terminator
    let target_len = target.len().saturating_sub(1);
    let target = &mut target[..target_len];

    for (source_char, target_char) in zip(source.encode_utf16(), target.iter_mut()) {
        *target_char = source_char as _;
    }

    // Fix possibly split surrogate pair at the end
    if let Some(last) = target.last_mut()
        && HIGH_SURROGATES.contains(last)
    {
        *last = 0;
    }
}
