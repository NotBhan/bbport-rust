# GPU porting ledger

Every file under `legacy/gpu` (2144 total), tracked against the Rust tree.

## Totals

| Status | Files |
|---|---:|
| ported | 14 |
| pending | 580 |
| vendored | 1520 |
| asset/build | 30 |

## By top-level folder

| Folder | Files | Ported | Pending | Vendored | Asset/build |
|---|---:|---:|---:|---:|---:|
| CMakeLists.txt | 1 | 0 | 0 | 0 | 1 |
| VENDOR.txt | 1 | 0 | 0 | 0 | 1 |
| bbgpu.h | 1 | 1 | 0 | 0 | 0 |
| bbnet | 156 | 0 | 154 | 0 | 2 |
| dlss_bridge | 4 | 1 | 1 | 0 | 2 |
| patches | 3 | 0 | 0 | 0 | 3 |
| shadps4 | 390 | 8 | 361 | 0 | 21 |
| shim | 65 | 4 | 61 | 0 | 0 |
| third_party | 1520 | 0 | 0 | 1520 | 0 |
| tools | 3 | 0 | 3 | 0 | 0 |

The authoritative per-file list is `PORTING.tsv` (path, status, Rust target).
