// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause
fn main() {
    // Barrier candidate: exactly one `b-*` feature, default "asm".
    const ALL: &[&str] = &[
        "asm", "none", "weak", "bbmask", "bbuse", "noinline", "volatile", "subtle-vol", "subtle-bb",
        "cmov", "ctutils", "crate",
    ];
    let vals: Vec<String> = ALL.iter().map(|v| format!("\"{v}\"")).collect();
    println!("cargo::rustc-check-cfg=cfg(barrier, values({}))", vals.join(", "));
    let on: Vec<&str> = ALL
        .iter()
        .copied()
        .filter(|v| std::env::var(format!("CARGO_FEATURE_B_{}", v.to_uppercase().replace('-', "_"))).is_ok())
        .collect();
    // subtle-bb also enables subtle's feature, not subtle-vol; b-weak may come from weak-barrier
    let b = match on.as_slice() {
        [] => "asm",
        [one] => one,
        many => panic!("more than one barrier feature: {many:?}"),
    };
    println!("cargo::rustc-cfg=barrier=\"{b}\"");
    println!("cargo::rustc-env=RLC_BARRIER={b}");
    #[cfg(feature = "ctgrind")]
    {
        let inc = std::env::var("VALGRIND_INC").expect("set VALGRIND_INC to valgrind's include dir");
        cc::Build::new().file("ctgrind/ctgrind.c").include(inc).compile("ctgrind");
        println!("cargo:rerun-if-changed=ctgrind/ctgrind.c");
    }
}
