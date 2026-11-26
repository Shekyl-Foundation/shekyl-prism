Local extraction harness, **not part of dev**. Copy `sio_dump.rs` to `shekyl-prism/examples/` in a
checkout of `origin/dev` (used: `470d537`, which contains #47, #48, #49, #50), then

    cargo build --release -p shekyl-prism --features signer --example sio_dump

(Rust 1.97.1 as pinned.) It reads rows `set n xn yn u0..u3 v0..v3 s xq yq` on stdin and prints, per row,
`admit=0`, `admit=1 inter=none`, or `admit=1 inter=ok|marked U0..U3 V0..V3 T` by calling only
`hazmat::ideal::{GenericHnf::from_norm_and_root, GuardedIdeal::admit, GuardedIdeal::intersection}`.
