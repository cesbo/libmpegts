# libmpegts - Copilot Instructions

## Project Overview

High-performance Rust library for MPEG-TS processing in highload systems.

## Maintenance Rule

AI agents must keep this file up to date:
- Update `## Implementation Status` when completing TODO items
- Add new public types and API changes to relevant sections
- When adding a new module, update the `## Architecture` directory tree

## Core principle

- **zero-copy parsing** - read data directly from buffers without deserialization.
- **std only** — the library requires `std`. `no_std` support is not a goal.
- **single-threaded** — the library is not designed for multi-threaded use. Each instance (e.g. `Psi`, `PesPacketizer`, `TsSlicer`) should be owned by a single thread.

## Architecture

```
src/
├── ts/        # TS packet: TsPacketRef, TsPacketMut, AdaptationFieldRef
├── psi/       # PSI tables: PAT, PMT, SDT, NIT, EIT, TDT, TOT
├── pes/       # PES packetization
├── mux/       # SPTS multiplexer: frame → TS packets with PSI/PCR
├── slicer/    # Stream slicing with sync recovery
└── utils/     # bits, crc32, bcd, mjd, textcode
```

## Key Design Pattern: `*Ref` Types

Zero-copy wrappers around byte slices with on-demand accessor methods:

```rust
// Wrap bytes, parse on access
pub struct PatSectionRef<'a>(&'a [u8]);

impl<'a> PatSectionRef<'a> {
    pub fn table_id(&self) -> u8 { self.0[0] }
    pub fn tsid(&self) -> u16 { u16::from_be_bytes([self.0[3], self.0[4]]) }
    pub fn version(&self) -> u8 { (self.0[5] & 0x3e) >> 1 }
}

// Validation via TryFrom
impl<'a> TryFrom<&'a [u8]> for PatSectionRef<'a> {
    type Error = PsiSectionError;
    fn try_from(data: &'a [u8]) -> Result<Self, Self::Error> {
        // Validate length, table_id, CRC32
    }
}
```

## Naming Conventions

| Suffix | Purpose | Example |
|--------|---------|---------|
| `*Ref` | Immutable byte wrapper | `TsPacketRef`, `PmtItemRef` |
| `*Mut` | Mutable byte wrapper | `TsPacketMut` |
| `*Builder` | PSI section builder | `PatBuilder`, `PmtBuilder` |
| `*Iter` | Iterator type | `PatItemIter`, `DescriptorIter` |
| `*_PID` | PID constant | `PAT_PID`, `SDT_PID`, `EIT_PID` |

## Error Handling

- Use `TryFrom` for fallible conversions
- Iterators yield `Result<ItemRef, Error>`
- Error enums: `PsiSectionError`, `PesPacketizerError`
- No panics; use `debug_assert!` for internal invariants

## Code Style

- `#[inline]` for small accessors
- Direct bit manipulation: `(value & MASK) >> SHIFT`
- Implement `AsRef<[u8]>` for underlying bytes access
- Builder pattern for complex structs: `PesHeader::new().with_pts().with_data_alignment()`

## PSI Section Generation: Builder Pattern

All PSI table builders follow a unified API with auto-splitting into multiple sections:

### Builder lifecycle

```rust
// 1. Create builder with required fields
let mut builder = PatBuilder::new(tsid);

// 2. Configure optional fields
builder.set_version(1);

// 3. Push items — builder auto-splits into new sections when capacity exceeded
builder.push(pnr, pid);  // may internally start a new section

// 4. Finalize — consumes builder, returns owned Sections
let sections = builder.finalize();  // patches headers, computes CRC32
// sections[0] — first (and typically only) section as &[u8]
```

### Auto-splitting rules

- Each builder knows its max section size (1024 for PAT/PMT/SDT/NIT, 4096 for EIT).
- `push()` checks remaining capacity. If the next item doesn't fit, the current section is sealed and a new one starts.
- `finalize(self)` consumes the builder, patches `section_number` and `last_section_number` in all sections, computes CRC32, returns owned `Sections`.
- Section numbering is 0-based and sequential.
- `begin_section()` is deferred (lazy) — called on first `push()` or in `finalize()` if no items were added. This allows `set_version()` and `set_descriptors()` to be called after `new()` but before the header is written.

### Simple tables (PAT)

```rust
let mut builder = PatBuilder::new(tsid);
builder.set_version(1);
builder.push(pnr, pid);
let sections = builder.finalize();
// sections[0] — first (and typically only) PAT section as &[u8]
```

### Tables with descriptors (PMT)

```rust
let mut builder = PmtBuilder::new(pnr, pcr_pid);
builder.set_version(1);
builder.set_descriptors(&program_descriptors);
builder.push(stream_type, pid, &es_descriptors);
let sections = builder.finalize();
```

## Multiplexer

Single-program (SPTS) VBR multiplexer. Accepts ES frames via push API, outputs interleaved TS packets with auto-generated PAT, PMT, and PCR.

### Public API

```rust
let mut mux = Multiplexer::new(tsid, pnr, pmt_pid);

// Register elementary streams. First stream becomes PCR PID.
let video = mux.add_stream(0x1B, &descriptors); // H.264 → returns stream index 0
let audio = mux.add_stream(0x0F, &descriptors); // AAC  → returns stream index 1

// Push ES frames
mux.push_frame(video, pts, dts, true, data);   // key frame
mux.push_frame(audio, pts, None, false, dcccccddnltata);

// Drain TS packets into caller buffer
let mut buf = [0u8; 188 * 200];
let n = mux.drain(&mut buf); // returns bytes written (multiple of 188)
```

### Internal types

| Type | Location | Purpose |
|------|----------|--------|
| `Multiplexer` | `mux/mod.rs` | Public API, owns streams, PSI packetizers, scheduler |
| `MuxStream` | `mux/mod.rs` | Per-stream: `PesPacketizer`, frame queue (`VecDeque<MuxFrame>`) |
| `MuxFrame` | `mux/mod.rs` | Queued frame: pts, dts, is_key_frame, data (`Vec<u8>`) |
| `Scheduler` | `mux/scheduler.rs` | VBR interleaving: decides next packet (PSI / PCR / ES) |

### Data flow

1. `push_frame()` → stores `MuxFrame` in `MuxStream.pending` queue
2. `drain()` → scheduler loop:
   - If key frame pending OR PSI interval elapsed → emit PAT + PMT via `PsiPacketizer`
   - If PCR interval elapsed → emit PCR-only packet (AF-only, no payload)
   - Pick next stream frame by earliest DTS → `PesPacketizer::set_frame()` → generate TS packets
   - Interleave packets across streams (weighted round-robin by pending packet count)
   - Write 188-byte packets into output buffer until full or all queued frames are drained
   - Return bytes written

### PCR generation

- PCR-only TS packet: AF-only (no payload), AF length = 183, PCR flag set
- PID = first registered ES stream PID (PCR PID)
- PCR value derived from PTS: `pcr = pts * 300` (90 kHz → 27 MHz)
- Default PCR interval: ~40 ms
- Helper: `write_pcr(packet, pid, cc, pcr)` in `ts/mod.rs`

### PAT/PMT insertion

- Before every key frame
- At configurable interval (default ~500 ms) if no key frame arrives
- Built using existing `PatBuilder` / `PmtBuilder`
- Packetized via `PsiPacketizer` (CC preserved across rebuilds)

### VBR interleaving

- No CBR null-stuffing; packets emitted only when data is available
- Scheduler picks streams in order of earliest DTS
- Video packets interleaved with audio to avoid bursts

## Implementation Status

### Done

- **TS packet**: `TsPacketRef`, `TsPacketMut`, `AdaptationFieldRef` — full read/write API
- **PSI parsing**: `PatSectionRef`, `PmtSectionRef`, `SdtSectionRef`, `NitSectionRef`, `EitSectionRef`, `TdtSectionRef`, `TotSectionRef`
- **PSI assembly**: `Psi` — stateful multi-packet section assembler with CC validation
- **PES packetization**: `PesPacketizer` with ring buffer output
- **Stream slicing**: `TsSlicer` with sync recovery
- **Descriptors**: `DescriptorRef`, `DescriptorsRef`, `DescriptorIter` — generic tag+data parsing
- **Sections**: `Sections` — owned collection of finalized PSI section slices (index access only, no iterator)
- **PatBuilder**: PAT section builder with auto-splitting and CRC32
- **PmtBuilder**: PMT section builder with program/ES descriptors and CRC32
- **PSI packetization**: `PsiPacketizer` — converts `Sections` into TS packets with CC tracking, `reset()` for periodic re-transmission
- **Utils**: CRC32, BCD, MJD, bit manipulation, textcode

### In Progress

- **Multiplexer**: SPTS VBR mux (`Multiplexer`, `MuxStream`, `MuxFrame`, `Scheduler`)

### TODO

- `SdtBuilder` builder with auto-splitting
- `NitBuilder` builder with auto-splitting
- `EitBuilder` builder with auto-splitting (4096 max section size)
- **PsiConfig methods**: `pat_sections()`, `pmt_sections()`, `sdt_sections()`, `nit_sections()`, `eit_sections()`
- **Multi-program multiplexer**: Extend `Multiplexer` for MPTS
- **Typed descriptors**: `ServiceDescriptorRef (0x48)`, `Iso639LanguageDescriptorRef (0x0A)`, `StreamIdentifierDescriptorRef (0x52)`, etc.
- **Descriptor generation**: Config-based descriptor building
