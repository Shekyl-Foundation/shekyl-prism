// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Retains the original P500-only fixture and benchmark entry point.
#![forbid(unsafe_code)]

#[path = "stage2c/driver.rs"]
mod driver;
#[path = "stage2c/reducer.rs"]
mod reducer;

fn main() {
    driver::run::<prism_field::P500, _, _>(
        include_str!("stage2c/vectors/p500.txt"),
        driver::mode(),
    );
}
