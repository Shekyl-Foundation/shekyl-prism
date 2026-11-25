# Working state (SHE-22, normalized Gram)

Resume notes for whoever picks this up; removed or folded into REPORT.md at the end.

- [x] Vendored a60fd06 crate (rust/), 1db834d originals (orig_*.rs), Python helpers (vendor/)
- [x] ngram.py model, gen_ng.py inputs + references (all asserts pass, 5 sets)
- [x] bounds_ng.py: width bounds, limb choice, E check
- [x] Rust: constructors (generic HNF Hensel/restoring, general), normalized reducer, check bin
- [x] Bench (setup / reduction / total; cold and warm), Memcheck
- [ ] REPORT.md, merge into research, INDEX row, delete branch
