# AGENTS.md

Guidance for coding agents working on `libmpegts`, a Rust library for MPEG-TS / DVB SI used in high-load systems.

Specs: ISO/IEC 13818-1 (MPEG-2 Systems), ETSI EN 300 468 (DVB SI) and ANSI/SCTE 35 (splice_info_section). Prefer those over guesswork when packing bitfields.

## Commands

```bash
cargo test                  # unit, integration, and doctests
cargo test --doc            # doctests only
cargo test --example ts_check --example ts_remux   # examples compile
cargo test --test pat -- --nocapture   # one integration test binary
rustfmt +nightly --edition 2024 --unstable-features --skip-children <changed files>
```

`rustfmt.toml` uses nightly-only options. Format only the files you change: `--skip-children` keeps rustfmt from descending into the modules a `mod.rs` declares.

No features, no workspace, no CI config in-tree. `Cargo.lock` is gitignored (library crate). Edition is `2024` (rustc 1.85+).

Do not add dependencies unless the task requires it. The only crate dependency is `textcode` (DVB character sets), re-exported as `libmpegts::utils::textcode`.

## Layout

```
src/lib.rs              public modules only
src/constants.rs        DVB delivery-system numeric constants (FEC, modulation, …)
src/ts/                 188-byte packet Ref/Mut, adaptation field, NULL and PCR packets, CcChecker
src/slicer/             TsSlicer: byte stream → TsPacketRef, sync recovery
src/psi/                section assemble / packetize / tables, SectionTable (table.rs), SCTE-35 (scte35.rs)
src/psi/descriptors/    one file per tag: x09.rs, x48.rs, …
src/pes/                PES header Ref/Mut/owned, packetizer, PTS/DTS
src/pcr/                27 MHz clock arithmetic
src/pcr/synth/          SPTS PCR synthesis / restamp / inject
src/mux/                SPTS multiplexer (PES + auto PAT/PMT/PCR)
src/utils/              crc32b, BCD, MJD, pack_bits!/unpack_bits!
tests/                  integration tests
tests/data/mod.rs       captured TS packets as byte arrays
examples/               ts_check, ts_remux
```

`src/psi/mod.rs` re-exports every table and every descriptor. Import from `libmpegts::psi::*` in tests; in library code import the sibling types you need.

## Architecture

### Zero-copy first

Parse types are newtypes over borrowed bytes. They must not copy the section or allocate on the parse path.

| Role | Name | Backing |
| --- | --- | --- |
| Read | `XxxSectionRef<'a>(&'a [u8])`, `TsPacketRef`, `PesHeaderRef`, `DescNNRef` | borrowed slice |
| In-place edit | `XxxSectionMut<'a>`, `TsPacketMut`, `PesHeaderMut` | `&mut [u8]` |
| Generate | `XxxBuilder` + `XxxConfig` → `Sections` | owned `Vec<u8>` |
| Encode descriptor | `DescNN` implementing `Descriptor` | writes into `&mut Vec<u8>` |

`TryFrom<&[u8]>` (or `TryFrom<DescriptorRef>`) is the parse entry point. Return `PsiSectionError` / `PesHeaderError`. Do not `unwrap` in those impls.

Iterators over table loops (`PatProgramIter`, `DescriptorIter`, …) yield `Result<T, PsiSectionError>` so a truncated last item is an error, not a panic. On error, consume the rest of the slice so the iterator stops.

### PSI pipeline

1. `TsSlicer::slice(&[u8])` → `TsPacketRef` (zero-copy except the one packet that straddles chunks).
2. `Psi::assemble(&packet)` — one `Psi` per PID. Completed sections live in `psi.sections()` only until the next `assemble` call.
3. `XxxSectionRef::try_from(section)` — validates `table_id`, length, CRC (except TDT, which has none) and every length field a getter slices by.
   `SectionTable::push(section)` tracks one long-form sub-table across repetitions (`Ignored` / `Unchanged` / `Updated` / `Reset`); it does not check the CRC, so call `check_crc32` first.
4. Build: `XxxBuilder::build(config) -> Sections`. Internals: `begin_section` / `push` / `seal_section` / `finalize_sections` (patches `section_length`, section numbers, CRC32).
5. On-wire: `PsiPacketizer::new(pid)` then `set_sections` / `next(&mut [u8; 188])`. Continuity counter survives `reset`.

`PmtSectionMut` and `EitSectionMut` wrap `PsiSectionMut`. After any field write call `update_crc32()`.

### Clocks

- PCR: 27 MHz, packed as `base * 300 + ext`, range `[0, PCR_NONE)` where `PCR_NONE = (1 << 33) * 300`. Use `pcr_delta` / `pcr_add` / `pcr_signed_delta`; never subtract raw PCR values.
- PTS/DTS: 90 kHz (`Timestamp`). `1 ms = 90` ticks. Mux PCR interval and PSI interval are in this domain.

`PcrSynth` and `Multiplexer` are SPTS-only. More than one PAT program disables synth (`PcrSynthPhase::MultiProgram`). Do not silently extend them to MPTS.

### Bit packing

Use `pack_bits!` / `unpack_bits!` from the crate root when encoding multi-field integers. Reserved bits are ones (`0b111`, `0b1111`, …) unless the spec says otherwise.

```rust
self.buffer.extend_from_slice(&pack_bits!(u16,
    reserved: 3 => 0b111,
    pid: 13 => program.pid,
));
```

## Conventions

- Match neighbouring code. Do not reformat files you are not changing.
- Ranges are written with spaces: `&buf[start .. end]`, `for i in 0 .. n`.
- Imports: crate-granularity, nested, trailing commas, `std` then `crate`.
- Public items get `///` rustdoc. Builders and packetizers should have a doctest when you add or change them.
- Errors implement `core::error::Error` + `Display` via `"message".fmt(f)`.
- `debug_assert!(pid < PID_NONE)` for builder PID invariants; do not panic in parse.
- `try_from` checks everything a getter indexes: `section_length` covers at least the header and the CRC, and every length field a getter slices by (`program_info_length`, network / bouquet descriptors length, TOT descriptors loop) stays inside the section. Item loops walked by iterators may stay lazy: an `Err`, then the iterator stops.
- Helpers that take values read from the stream (PCR arithmetic, MJD, BCD) never panic: wrap on the PCR ring, saturate, or return a neutral value.
- Descriptor payload length is a `u8`. `encode` must return `InvalidDescriptorLength` when `data_len > 0xff`, and must not write a partial descriptor.
- No new `unsafe` unless there is no safe equivalent. The only existing `unsafe` is in `TsSlicerIter`, converting a 188-byte slice to `&[u8; PACKET_SIZE]`.
- Do not introduce `async`, interior mutability, or extra traits for things that are already functions on the newtype.
- Keep parse/hot paths allocation-free (`TsSlicer`, `TsPacketRef`, `Psi::assemble`, `PcrSynth` process). Allocate only in builders, mux queues, and tests.

### Naming

- Tables: `PatSectionRef`, `PatBuilder`, `PatConfig`, `PatProgram`.
- Descriptors: file `src/psi/descriptors/x{tag:02x}.rs`, types `Desc48Ref` / `Desc48`, `pub const TAG: u8 = 0x48` on the Ref.
- TS: `PACKET_SIZE = 188`, `SYNC_BYTE = 0x47`, `PID_NULL = 0x1FFF`, `PID_NONE = 8192`.

### Commits

`module: short imperative description`

Examples: `psi/eit: EitBuilder section generator`, `pcr/synth: patch packet in place`.

A release bump is its own commit, `version up`, touching only `Cargo.toml`. The maintainer publishes.

## Tests

Two layers:

1. **Unit tests** (`#[cfg(test)]` in the same file): builders, CRC, descriptor parse/encode/reject. Names are short verbs: `builds_empty_nit`, `parses_service_fields`, `rejects_wrong_tag`.
2. **Integration tests** (`tests/*.rs`): captured packets in `tests/data/mod.rs`, sliced and assembled. Names use a `test_` prefix: `test_parse_pat`, `test_build_pat_roundtrip`.

Typical table parse test:

```rust
let mut psi = Psi::default();
TsSlicer::new().slice(data::PAT).for_each(|p| psi.assemble(&p));
let section = psi.sections().first().expect("PAT section expected");
let pat = PatSectionRef::try_from(section).expect("Valid PAT section");
```

Round-trip: `Builder::build(config)` → `SectionRef::try_from(&sections[0][..])` → assert fields.

Descriptor tests (see `src/psi/descriptors/x48.rs`): parse happy path, empty/optional fields, wrong tag, truncated lengths, trailing bytes, encode, encode overflow. Helpers `descriptor()` / `first()` stay local to the test module.

`PcrSynth` tests live in `src/pcr/synth/tests.rs` (included from `mod.rs`). They are large and scenario-based; extend them rather than inventing a parallel harness.

Doctests in `///` examples may `.unwrap()`. Library parse code may not.

## Adding a descriptor

1. Create `src/psi/descriptors/xNN.rs` following `x09.rs` (simple) or `x48.rs` (text).
2. `DescNNRef`: `TAG`, getters over `self.0` (payload only, tag+length already stripped), `TryFrom<DescriptorRef>`.
3. `DescNN` (if encoding): public fields, `impl Descriptor` writing `tag`, `len`, payload.
4. `mod xNN;` + `pub use xNN::*;` in `src/psi/descriptors/mod.rs`.
5. Unit tests in the same file. Use `constants.rs` for delivery-system enums.

## Adding a PSI table

Copy a same-shape table (`pat.rs` for a simple loop, `pmt.rs` if you need `SectionMut`, `tdt.rs` for short-form without CRC).

Required pieces: PID/table_id constants, `XxxSectionRef` + `TryFrom` (with the length checks above), iterator over the loop, `XxxConfig` + `XxxBuilder::build -> Sections` with a doctest when something generates the table (SCTE-35 is parse-only and says so in its module doc). Wire it through `src/psi/mod.rs`. Add `tests/<table>.rs` with a captured packet in `tests/data/mod.rs` when you have one.

TDT has no CRC. TOT and the SCTE-35 splice_info_section are short-form but do have a CRC. Do not call `check_crc32` on TDT. `SectionTable` takes long-form sections only.

## Out of scope unless asked

- MPTS mux or multi-program PCR synth
- `no_std`
- New Cargo features
- Rewriting working parse code to owned structs
- Running rustfmt/clippy as a drive-by on unrelated files
- Expanding public API “for completeness” without a caller
