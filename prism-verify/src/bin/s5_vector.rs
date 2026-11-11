// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Derive the §5 challenge at `e′ = e − 2`, and write a harness response
//! into the vector file after the sign normalization.
//!
//! `index <I|III|V> <A-hex>` prints `j` and decimal `s`.
//! `commit <I|III|V> <response>` reads the harness lines on stdin and writes
//! the vector. The response file is the harness stdout plus an `A` line.

use std::env;
use std::fs;
use std::io::{self, Read};
use std::process::ExitCode;

use prism_field::{Field, Level, Level1, Level3, Level5};
use prism_verify::signature::{canonical_sign, AffinePoint};
use prism_verify::smallest_at_exponent;

const MESSAGE: &[u8] = b"shekyl-prism-s5";

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        eprintln!("usage: s5_vector index <level> <A-hex> | commit <level> <out>");
        return ExitCode::from(2);
    };
    match command.as_str() {
        "index" => {
            let (level, coefficient) = match (args.next(), args.next()) {
                (Some(level), Some(coefficient)) => (level, coefficient),
                _ => {
                    eprintln!("usage: s5_vector index <level> <A-hex>");
                    return ExitCode::from(2);
                }
            };
            dispatch(&level, &coefficient, None)
        }
        "commit" => {
            let (level, path) = match (args.next(), args.next()) {
                (Some(level), Some(path)) => (level, path),
                _ => {
                    eprintln!("usage: s5_vector commit <level> <out>");
                    return ExitCode::from(2);
                }
            };
            let mut text = String::new();
            if io::stdin().read_to_string(&mut text).is_err() {
                eprintln!("could not read the harness response");
                return ExitCode::FAILURE;
            }
            dispatch(&level, &text, Some(&path))
        }
        _ => {
            eprintln!("unknown command {command}");
            ExitCode::from(2)
        }
    }
}

fn dispatch(level: &str, payload: &str, out: Option<&str>) -> ExitCode {
    match level {
        "I" | "NIST-I" => run::<Level1>(payload, out),
        "III" | "NIST-III" => run::<Level3>(payload, out),
        "V" | "NIST-V" => run::<Level5>(payload, out),
        _ => {
            eprintln!("level is I, III, or V");
            ExitCode::from(2)
        }
    }
}

fn run<L: Level>(payload: &str, out: Option<&str>) -> ExitCode {
    if let Some(path) = out {
        commit::<L>(payload, path)
    } else {
        index::<L>(payload)
    }
}

fn index<L: Level>(coefficient: &str) -> ExitCode {
    let Ok(bytes) = hex_bytes(coefficient.trim()) else {
        eprintln!("A is not hex");
        return ExitCode::FAILURE;
    };
    let exponent = L::E - 2;
    let Ok((index, scalar)) = smallest_at_exponent::<L>(&bytes, MESSAGE, exponent) else {
        eprintln!("no prime degree at e'");
        return ExitCode::FAILURE;
    };
    println!("{}", index);
    println!("{}", decimal(scalar.as_bytes()));
    ExitCode::SUCCESS
}

fn commit<L: Level>(text: &str, path: &str) -> ExitCode {
    let Ok(bytes) = field_line(text, "A") else {
        eprintln!("missing A");
        return ExitCode::FAILURE;
    };
    let exponent = L::E - 2;
    let Ok((index, scalar)) = smallest_at_exponent::<L>(&bytes, MESSAGE, exponent) else {
        eprintln!("no prime degree at e'");
        return ExitCode::FAILURE;
    };
    let Ok(curve) = ext::<L>(text, "E") else {
        eprintln!("E does not decode");
        return ExitCode::FAILURE;
    };
    let Ok(px) = ext::<L>(text, "Px") else {
        eprintln!("Px does not decode");
        return ExitCode::FAILURE;
    };
    let Ok(py) = ext::<L>(text, "Py") else {
        eprintln!("Py does not decode");
        return ExitCode::FAILURE;
    };
    let Ok(qx) = ext::<L>(text, "Qx") else {
        eprintln!("Qx does not decode");
        return ExitCode::FAILURE;
    };
    let Ok(qy) = ext::<L>(text, "Qy") else {
        eprintln!("Qy does not decode");
        return ExitCode::FAILURE;
    };
    let Ok(vk_px) = field_line(text, "VkPx") else {
        eprintln!("missing VkPx");
        return ExitCode::FAILURE;
    };
    let Ok(vk_py) = field_line(text, "VkPy") else {
        eprintln!("missing VkPy");
        return ExitCode::FAILURE;
    };
    let Ok(vk_qx) = field_line(text, "VkQx") else {
        eprintln!("missing VkQx");
        return ExitCode::FAILURE;
    };
    let Ok(vk_qy) = field_line(text, "VkQy") else {
        eprintln!("missing VkQy");
        return ExitCode::FAILURE;
    };
    let Ok(vk_dx) = field_line(text, "VkDx") else {
        eprintln!("missing VkDx");
        return ExitCode::FAILURE;
    };
    let mut point_p = AffinePoint::<L> { x: px, y: py };
    let mut point_q = AffinePoint::<L> { x: qx, y: qy };
    if !canonical_sign::<L>(point_p.y) {
        point_p.y = point_p.y.negate();
        point_q.y = point_q.y.negate();
    }
    let body = format!(
        "level {}\nmessage shekyl-prism-s5\nj {}\ns {}\nq {}\nA {}\nE {}\nPx {}\nPy {}\nQx {}\nQy {}\nVkPx {}\nVkPy {}\nVkQx {}\nVkQy {}\nVkDx {}\n",
        L::NAME,
        index,
        decimal(scalar.as_bytes()),
        degree_decimal(exponent, scalar.as_bytes()),
        hex_bytes_encode(&bytes),
        hex_bytes_encode(&L::ext_encode(curve)),
        hex_bytes_encode(&L::ext_encode(point_p.x)),
        hex_bytes_encode(&L::ext_encode(point_p.y)),
        hex_bytes_encode(&L::ext_encode(point_q.x)),
        hex_bytes_encode(&L::ext_encode(point_q.y)),
        hex_bytes_encode(&vk_px),
        hex_bytes_encode(&vk_py),
        hex_bytes_encode(&vk_qx),
        hex_bytes_encode(&vk_qy),
        hex_bytes_encode(&vk_dx),
    );
    if fs::write(path, body).is_err() {
        eprintln!("could not write {path}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

fn degree_decimal(exponent: u32, scalar: &[u8]) -> String {
    let bit = (exponent as usize) * 2 - 1;
    let mut power = vec![0u8; bit / 8 + 1];
    power[bit / 8] |= 1 << (bit % 8);
    let square = square_le(scalar);
    let mut borrow = 0i16;
    for (limb, sub) in power
        .iter_mut()
        .zip(square.iter().chain(std::iter::repeat(&0)))
    {
        let diff = *limb as i16 - *sub as i16 - borrow;
        if diff < 0 {
            *limb = (diff + 256) as u8;
            borrow = 1;
        } else {
            *limb = diff as u8;
            borrow = 0;
        }
    }
    decimal(&power)
}

fn square_le(scalar: &[u8]) -> Vec<u8> {
    let mut out = vec![0u8; scalar.len() * 2];
    for (i, left) in scalar.iter().enumerate() {
        let mut carry = 0u16;
        for (j, right) in scalar.iter().enumerate() {
            let slot = &mut out[i + j];
            let product =
                u32::from(*slot) + u32::from(*left) * u32::from(*right) + u32::from(carry);
            *slot = product as u8;
            carry = (product >> 8) as u16;
        }
        let mut extra = carry;
        let mut at = i + scalar.len();
        while extra > 0 {
            if at == out.len() {
                out.push(0);
            }
            let sum = u16::from(out[at]) + extra;
            out[at] = sum as u8;
            extra = sum >> 8;
            at += 1;
        }
    }
    out
}

fn ext<L: Level>(text: &str, label: &str) -> Result<L::Ext, ()> {
    let bytes = field_line(text, label)?;
    L::ext_decode(&bytes).ok_or(())
}

fn field_line(text: &str, label: &str) -> Result<Vec<u8>, ()> {
    let prefix = format!("{label} ");
    let line = text
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .ok_or(())?;
    hex_bytes(line.trim())
}

fn hex_bytes(text: &str) -> Result<Vec<u8>, ()> {
    if !text.len().is_multiple_of(2) {
        return Err(());
    }
    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).map_err(|_| ()))
        .collect()
}

fn hex_bytes_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn decimal(bytes: &[u8]) -> String {
    let mut digits = vec![0u8];
    for byte in bytes.iter().rev() {
        let mut carry = u32::from(*byte);
        for digit in &mut digits {
            carry += u32::from(*digit) << 8;
            *digit = (carry % 10) as u8;
            carry /= 10;
        }
        while carry > 0 {
            digits.push((carry % 10) as u8);
            carry /= 10;
        }
    }
    while digits.len() > 1 && digits.last() == Some(&0) {
        digits.pop();
    }
    digits
        .iter()
        .rev()
        .map(|digit| char::from(b'0' + digit))
        .collect()
}
