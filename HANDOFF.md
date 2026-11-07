# Handoff — the Rust port

Everything below is what the next session needs to keep going without re-deriving it. The
documents that already exist and should stay the source of truth are `RUST_PORT.md` (milestones,
what is ported, CLI) and `gpu/PORTING.md` + `gpu/PORTING.tsv` (the per-file ledger). This note is
the working method, the standing rules, and the terrain.

## What this tree is

`rust_launcher/` is the Rust reimplementation of the port: the crate root is `lib.rs`/`main.rs`,
the launcher lives in `launcher/`, the C runtime port under `src/`, and the C++ renderer port
under `gpu/`. The original sources moved to `legacy/` (C in `legacy/src/`, C++ in
`legacy/gpu/`) and are the only specification — the port does not invent behaviour.

Two milestones are done (launcher core, remaining launcher Python). Milestone 3 is the C/C++
port and is where all remaining work is. Inventory is 2144 files: **590 ported, 0 pending,
1522 vendored, 32 build/assets**. `legacy/` and `target/` are git-ignored; **the crate directory
is not a git repository**, so nothing is version controlled here and `cargo fix`/`cargo clippy
--fix` refuse to run without `--allow-no-vcs`. Edit by hand or use `--allow-dirty --allow-no-vcs`.

## Where the truth lives

| Thing | Where |
|---|---|
| Per-file status and Rust target | `gpu/PORTING.tsv` (tab separated: path, status, note) |
| Human summary of the ledger | `gpu/PORTING.md` |
| Milestones, build, CLI | `RUST_PORT.md` |
| The specification | `legacy/gpu/**` and `legacy/src/**` |

Statuses in the ledger are `ported`, `pending`, `vendored` (third-party kept as a dependency),
`asset/build`. **A row is only marked `ported` when the file is complete, wired into `mod.rs`,
compiling, and has tests.** Half-ported files stay `pending` and say so in `RUST_PORT.md`.

## The loop

1. Pick the next unblocked file. Work in dependency order — a header before its `.cpp`, and a
   type before the file that constructs it.
2. Read the C++ source in full. Do not port from the header alone; the bodies are where the
   quirks are.
3. Write the Rust next to its siblings (`shadps4/shader_recompiler/ir/ir_emitter.h` →
   `gpu/shader_recompiler/ir/ir_emitter.rs`), declare it in that directory's `mod.rs`, and add
   tests that pin observable behaviour — opcode sequences, flag words, boundary panics, parsed
   field values — not just that functions return something.
4. Run the gates and fix everything before reporting:

```bash
cargo fmt && cargo fmt --check
cargo clippy --all-targets        # must report nothing
cargo test                        # all green
cargo build --release
```

5. Update `gpu/PORTING.tsv` (one row per source file, including the `.cpp` separately from its
   `.h`) and the counts plus test total in `RUST_PORT.md`.
6. Report: what was ported, the adaptations worth knowing, the gates, the new ledger counts, and
   the single next-up file.

## Standing rules

- **No unnecessary multiline comments.** No per-file explanatory header essays, no restating what
  the code says. A short module doc naming the source file is right; a paragraph of rationale
  inside a function is not. Comments belong where the code cannot explain itself — a C++ quirk
  kept on purpose, a deliberate deviation.
- **Preserve C++ behaviour, including its quirks.** Do not tidy, do not "fix" what looks like a
  bug in the original, and do not add error handling for states the C++ cannot reach. If the C++
  is wrong-but-shipped, the port is the same and the deviation is a note, not a change.
- **Logging is dropped.** `LOG_DEBUG`/`LOG_WARNING`/`LOG_ERROR` have no counterpart; where a
  warning carried information the caller needs, expose a predicate instead (for example
  `is_unimplemented_conditional_branch`).
- **Assertions become panics with the C++'s wording.** `ASSERT_MSG` texts are kept verbatim, and
  `ThrowInvalidType` keeps its message shape (function and line come from `Location::caller()`).
- **Pointers become owned data or handles.** `Block*` is a `BlockRef` (`Rc<RefCell<Block>>`),
  and where the C++ dereferenced a guest pointer the port takes a reader
  (`&dyn Fn(u64) -> u32`) instead.
- **Unions that Rust cannot read across become enums or read-modify-write setters** on the
  shared word (`InstControl::set_raw` plus `set_count`/`set_size`/…). Say so in a one-line note
  when the observable difference is real.
- **Generated tables are generated.** No hand-typed 2000-entry arrays: `tools/gen_gcn_opcodes.py`
  and `tools/gen_gcn_formats.py` produce the Rust and self-check with `--check`; regenerate and
  keep them rustfmt-clean.
- **C++ overloads become distinct names**, one per arity or per type: `composite_shuffle3`,
  `imm32_signed`, `get_scalar_reg_f32`, `shared_atomic_inc64`. Prefer names that say which
  overload it is over generic suffixes.

## Landmines already handled (do not "fix" these)

- **Aliasing enumerators.** C++ `enum class` tables where two names share a value (23 pairs in
  the GCN opcode tables) cannot be Rust enums: they are `#[repr(transparent)] u32` newtypes with
  associated constants, and name lookup returns the *first declared* name to match `enum_name`.
- **`updateInstructionMeta`** copies `src_count` but leaves `dst_count` as the decoder set it.
  VOP3 destination counts come from the encoding range `V_ADD_I32..V_DIV_SCALE_F64`, so
  `V_MAD_F32` gets two destinations. Both are tested.
- **`ResolveSetPcTarget`'s `< 3` guard** drops targets in the first three instructions.
- **`HwGeometryRuntimeInfo` equality** tests non-zero-ness of invocation counts, not equality.
- **DS "dual" and "stride" instruction sets are disjoint**; `InstructionFormat` masks are checked
  widest-first; `GetEncodingLength`/`DecodeOpcode` were declared and never defined in the C++, so
  they are not ported.
- **Flags are a `u64` word** end to end (`Inst::new`, `Block::PrependNewInst`). That is what lets
  `DebugPrint`'s argument count and the GDS bit travel where they do in C++.
- **`Program` shares `Info` through `Rc<RefCell<Info>>`** because passes write back into it
  (`inject_clip_distance_attributes` bumps `num_inputs`). `DumpProgram` is split into
  `dump_program` (writes files) and `dump_program_text` (pure, testable); the
  `EmulatorSettings.IsDumpShaders()` gate is the caller's, since there is no settings object.
- **`IREmitter`'s insertion point** is `Option<usize>`: `None` is the C++'s `end()`, and a
  `Some(i)` insert leaves the index stepping over the instruction just written, matching an
  iterator that keeps pointing at the old instruction. There is a test for it.
- **`ir_passes.h`, `position.h`, `structured_control_flow.cpp`** all call into the emitter, which
  is why they were blocked until it landed.

## Where the work stands

The shader compiler front end and IR are nearly done: GCN opcodes, instruction layout, control
flow graph, the format tables, the decoder, the copy and fetch shader parsers, the profile and
register headers, `runtime_info`, the AMD resource descriptions, `Info`, the SRT pass, the
expression-elimination, constant-propagation and dead-code passes, SSA repair, phi lowering,
clip-distance injection, the interpolated-integer fix, shared-memory simplification, user clip
plane lowering, hardware intrinsic lowering, the resource discovery types and instruction
predicates, the value swizzle/number conversions, resource patching (descriptor binding,
addresses, image arguments, GDS ring), extended-userdata flattening (address folding, pointer
graph, flatbuf offsets), the SRT value-numbering table, the hull/domain tessellation transforms
and the tessellation-constant preprocess, the value/instruction pattern matcher, phi
simplification, `ir_emitter`, `ir/program`, the structured control flow builder (`BuildASL`),
and the position export. The GCN front end is complete: every translate category (including
`vector_alu`) is ported, and the driver (`recompiler.rs`) plus the permutation key
(`specialization.rs`) are in place. The SPIR-V backend is complete too: the local Sirit
stand-in, the generated `spv` constants, the emit context, every emitter and the
driver/dispatcher are ported. There are no pending `shader_recompiler` rows left.

## Next up, in order

1. `video_core/amdgpu` — the remaining register headers (`regs_*.h` bitfield unions,
   `pm4_cmds.h` packet bodies) and `liverpool.cpp`, then `texture_cache`/`buffer_cache` and
   `renderer_vulkan`.
2. Elsewhere: the runtime C backends (`runtime_sema.c`, `runtime_rwlock.c`, `runtime_file.c`,
   `runtime_memory.c`, `runtime_thread.c`, `runtime_services.c`, `runtime_kernel.c`, then
   `runtime.c`/`probe.c`), then the `shim`/`bbnet` layers.

No rows are pending. `http.cpp` is ported to `gpu/core/libraries/network/http.rs`;
the two FidelityFX headers are vendored dependencies, and `vendor-changes.diff`
and `LICENSE` are recorded as assets.

## Rust GUI

`gui.rs` is the native GTK4 (`gtk4` crate) launcher: readiness rows, game-folder
picker, settings.json/bbport.ini pages (Display, Graphics, Upscaling, Performance,
Compatibility, Controller, Game & Data, Launcher), trophy report, diagnostics and a
**Launch Game** button that spawns the port's `run.sh` with `launcher_core::process`.
Port discovery walks ancestors and sibling `linux_launcher`/`windows_laucnher` trees so
it works from `target/release`. Run with `./run-rust-gui.sh` or
`cargo run --bin bbport-gui`; `BB_PORT_DIR` picks the port tree explicitly.

## Practical notes

- Run one file's tests with `cargo test --lib <module path>`, for example
  `cargo test --lib ir::ir_emitter`.
- Tests do not share state safely by default: two tests that touch the same thread-local or
  process-wide cache will race under the parallel test harness. Give them a lock (there is
  precedent in the shim's `bbport_map_features` tests).
- Where a port pins a table against its C++ original, it does so with `include_str!` on the
  header and asserts the enumerator count. Keep that pattern for new generated tables.
- The port's `Type` lattice, `TypedValue` masks and the `FromInst` trait in `ir_emitter.rs` are
  deliberate: they reproduce the C++'s implicit conversions, including `TypedValue(IR::Inst*)`
  skipping the type assertion that `TypedValue(const Value&)` performs.
- `RUST_PORT.md` ends with a stray `cmd --resume <session id>` fragment from an earlier session.
  It is not content; drop it when next editing that file.

## Audit 2026-11-07

Safety/robustness pass completed: Zip Slip rejection in co-op extraction, bounded ZIP directory
allocation, extractor stderr draining, destroyed-cond-handle rejection, pthread handle-lifetime
fixes, avplayer guest-string validation, slot-vector mmap assertion, bind-helper disabled path,
equeue guest-pointer boundaries, and rasterizer-lock reentrancy. Test suite: 1135 passed.
