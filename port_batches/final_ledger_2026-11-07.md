# Final ledger state — 2026-11-07

Live command: `awk -F'\t' '$2=="ported"{p++} $2=="pending"{q++} $2=="vendored"{v++} $2=="asset"{a++} END{print "ported",p,"pending",q,"vendored",v,"asset",a}' gpu/PORTING.tsv`

Result:

```
ported 590 pending  vendored 1522 asset 2
```

- `ported` is 590 Rust ports (target: at least 300).
- `pending` is empty.
- The last source row closed this turn is
  `bbnet/vendor/shadps4/core/libraries/network/http.cpp` →
  `gpu/core/libraries/network/http.rs`.
- `ffx_a.h` and `ffx_fsr1.h` are third-party FidelityFX headers and are
  `vendored`; `vendor-changes.diff` and `LICENSE` are `asset` rows.

Gates (run after the final ledger update):

- `cargo fmt --check` — clean
- `cargo clippy --all-targets` — clean
- `cargo test` — 1126 lib tests + 2 integration tests pass
- `cargo build --release` — succeeds
