// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

fn main() {
    #[cfg(feature = "memcheck")]
    {
        let source = "ctgrind/ctgrind.c";
        println!("cargo:rerun-if-changed={source}");
        cc::Build::new().file(source).compile("ctgrind");
    }
}
