// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! `verify::<Level5>` on arbitrary bytes. A panic is a failed nightly.

#![no_main]

use libfuzzer_sys::fuzz_target;
use prism_field::Level5;
use prism_verify::verify;

fuzz_target!(|data: &[u8]| {
    let _ = verify::<Level5>(data, data, data);
    let (public_key, message, signature) = split(data);
    let _ = verify::<Level5>(public_key, message, signature);
});

fn split(data: &[u8]) -> (&[u8], &[u8], &[u8]) {
    if data.len() < 4 {
        return (data, b"", b"");
    }
    let public_key_len = usize::from(u16::from_le_bytes([data[0], data[1]]));
    let signature_len = usize::from(u16::from_le_bytes([data[2], data[3]]));
    let body = &data[4..];
    let public_key_len = public_key_len.min(body.len());
    let signature_len = signature_len.min(body.len() - public_key_len);
    (
        &body[..public_key_len],
        &body[public_key_len + signature_len..],
        &body[public_key_len..public_key_len + signature_len],
    )
}
