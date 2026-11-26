// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Native reducer fixtures at all five PRISM signing parameter sets.
#![forbid(unsafe_code)]

mod driver;
mod reducer;

use prism_field::{P248, P324, P376, P500, P664};

fn main() {
    let bench = std::env::args().nth(1).as_deref() == Some("bench");
    driver::run::<P248, _, _>(include_str!("vectors/p248.txt"), bench);
    driver::run::<P324, _, _>(include_str!("vectors/p324.txt"), bench);
    driver::run::<P376, _, _>(include_str!("vectors/p376.txt"), bench);
    driver::run::<P500, _, _>(include_str!("vectors/p500.txt"), bench);
    driver::run::<P664, _, _>(include_str!("vectors/p664.txt"), bench);
}
