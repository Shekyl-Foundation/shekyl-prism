// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
fn main() {
    #[cfg(feature = "ctgrind")]
    {
        let inc = std::env::var("VALGRIND_INC").expect("set VALGRIND_INC to valgrind's include dir");
        cc::Build::new().file("ctgrind/ctgrind.c").include(inc).compile("ctgrind");
        println!("cargo:rerun-if-changed=ctgrind/ctgrind.c");
    }
}
