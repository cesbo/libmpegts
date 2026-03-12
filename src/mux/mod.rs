//! MPEG-TS multiplexer.
//!
//! [`Multiplexer`] accepts elementary stream frames, wraps them into PES,
//! and writes MPEG-TS packets into a caller-provided buffer via [`Multiplexer::drain`].
//! PAT, PMT, and PCR are emitted automatically. Streams are scheduled by DTS,
//! or by PTS when DTS is absent.
//!
//! # Example
//!
//! ```rust
//! use libmpegts::{
//!     mux::{Multiplexer, MuxService, MuxStream, MuxFrame},
//!     ts::PACKET_SIZE,
//! };
//!
//! let mut mux = Multiplexer::new(1);
//! mux.add_service(&MuxService {
//!     program_number: 1,
//!     pmt_pid: 256,
//!     pcr_pid: 101,
//!     program_descriptors: Vec::new(),
//!     service_descriptors: Vec::new(),
//!     streams: vec![
//!         MuxStream {
//!             stream_type: 0x1B,
//!             elementary_pid: 101,
//!             stream_descriptors: Vec::new(),
//!         },
//!         MuxStream {
//!             stream_type: 0x0F,
//!             elementary_pid: 102,
//!             stream_descriptors: Vec::new(),
//!         },
//!     ],
//! });
//!
//! let video = mux.stream_index(101).unwrap();
//!
//! mux.push_frame(
//!     video,
//!     MuxFrame {
//!         data: vec![0u8; 1024],
//!         is_key_frame: true,
//!         pts_dts: Some((0, None).into()),
//!     },
//! );
//!
//! let mut buf = [0u8; PACKET_SIZE * 8];
//! let _n = mux.drain(&mut buf);
//! ```

use std::collections::VecDeque;

use crate::{
    pes::{
        EsFrame,
        PesHeader,
        PesPacketizer,
        PtsDts,
        STREAM_ID_AUDIO,
        STREAM_ID_PRIVATE_1,
        STREAM_ID_VIDEO,
        Timestamp,
    },
    psi::{
        PAT_PID,
        PatBuilder,
        PatConfig,
        PatProgram,
        PmtBuilder,
        PmtConfig,
        PmtStream,
        PsiPacketizer,
    },
    ts::PACKET_SIZE,
};

const PCR_DELAY: Timestamp = Timestamp::new(700 * 90); // 700ms delay
const PCR_INTERVAL: u64 = 40 * 90; // 40ms in 90kHz ticks
const PSI_INTERVAL: u64 = 500 * 90; // 500ms in 90kHz ticks

/// Elementary stream configuration for [`MuxProgram`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuxStream {
    /// MPEG-TS stream type (e.g. 0x1B for H.264, 0x0F for AAC)
    pub stream_type: u8,
    /// PID assigned to this stream
    pub elementary_pid: u16,
    /// Raw ES-level descriptors for PMT generation
    pub stream_descriptors: Vec<u8>,
}

/// Complete single-service mux configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuxService {
    /// MPEG program number
    pub program_number: u16,
    /// PID carrying the PMT for this program
    pub pmt_pid: u16,
    /// PID carrying PCR packets. Must match one of `streams[*].elementary_pid`
    pub pcr_pid: u16,
    /// Raw PMT program descriptors
    pub program_descriptors: Vec<u8>,
    /// Reserved for SDT generation
    pub service_descriptors: Vec<u8>,
    /// Elementary streams
    pub streams: Vec<MuxStream>,
}

/// Queued ES frame waiting to be packetized.
pub struct MuxFrame {
    /// ES frame payload
    pub data: Vec<u8>,
    /// Marks this frame as a key frame (for video) or access unit start (for audio)
    pub is_key_frame: bool,
    /// Presentation and Decoding timestamps (90 kHz clock)
    pub pts_dts: Option<PtsDts>,
}

#[derive(Clone, Copy)]
struct ActiveFrame {
    timestamp: Option<Timestamp>,
    pending_key_psi: bool,
    pending_key_pcr: bool,
}

/// Per-stream state inside the multiplexer.
struct ElementaryStream {
    pmt_stream: PmtStream,
    stream_id: u8,
    packetizer: PesPacketizer,
    pending: VecDeque<MuxFrame>,
    active: Option<ActiveFrame>,
}

impl ElementaryStream {
    /// Creates a new stream with the given type and PID.
    ///
    /// - `stream_type` - MPEG-TS stream type (e.g. 0x1B for H.264, 0x0F for AAC)
    /// - `pid` - PID assigned to this stream (must be unique and >= 0x20 and < 0x1FFF)
    fn new(stream_id: u8, pmt_stream: PmtStream) -> Self {
        let pid = pmt_stream.elementary_pid;
        Self {
            pmt_stream,
            stream_id,
            packetizer: PesPacketizer::new(pid),
            pending: VecDeque::new(),
            active: None,
        }
    }

    fn push_frame(&mut self, frame: MuxFrame) {
        // TODO: support frames without PTS/DTS
        if frame.pts_dts.is_none() {
            return;
        }

        self.pending.push_back(frame);
    }

    /// Feed the next queued frame into the packetizer.
    fn load_next_frame(&mut self) {
        debug_assert!(self.active.is_none());

        let Some(frame) = self.pending.pop_front() else {
            return;
        };

        let timestamp = frame.pts_dts.map(|ts| ts.timestamp());

        let mut header = PesHeader::new(self.stream_id).with_data_alignment(frame.is_key_frame);
        if let Some(pts_dts) = frame.pts_dts {
            header = header.with_pts_dts(pts_dts);
        }

        let es_frame = EsFrame {
            header,
            payload: frame.data,
            rai: frame.is_key_frame,
        };

        self.packetizer.set_frame(es_frame);
        self.active = Some(ActiveFrame {
            timestamp,
            pending_key_psi: frame.is_key_frame,
            pending_key_pcr: frame.is_key_frame && timestamp.is_some(),
        });
    }
}

#[derive(Clone, Copy)]
enum MultiplexerState {
    Idle,
    EmitPat,
    EmitPmt,
    EmitPcr(Timestamp),
    EmitFrame(usize),
}

/// Single-program (SPTS) VBR multiplexer.
///
/// Accepts ES frames via [`push_frame`](Self::push_frame) and produces
/// whole-frame MPEG-TS output with auto-generated PAT, PMT, and PCR
/// via [`drain`](Self::drain).
pub struct Multiplexer {
    tsid: u16,
    pat_packetizer: PsiPacketizer,

    program_number: u16,
    pmt_pid: u16,
    pmt_descriptors: Vec<u8>,
    pmt_packetizer: PsiPacketizer,

    state: MultiplexerState,
    psi_dirty: bool,

    /// Registered elementary streams.
    streams: Vec<ElementaryStream>,

    last_pcr_timestamp: Option<Timestamp>,
    last_psi_timestamp: Option<Timestamp>,
}

impl Multiplexer {
    /// Creates a new single-program multiplexer.
    pub fn new(tsid: u16) -> Self {
        Self {
            tsid,
            pat_packetizer: PsiPacketizer::new(PAT_PID),

            program_number: 1,
            pmt_pid: 256,
            pmt_descriptors: Vec::new(),
            pmt_packetizer: PsiPacketizer::new(256),

            state: MultiplexerState::Idle,
            psi_dirty: true,

            streams: Vec::new(),
            last_pcr_timestamp: None,
            last_psi_timestamp: None,
        }
    }

    /// Adds single service into mux
    pub fn add_service(&mut self, service: &MuxService) {
        for stream in &service.streams {
            let stream_id = match stream.stream_type {
                0x02 | 0x1B | 0x24 | 0x33 => self.next_stream_id(STREAM_ID_VIDEO, 0x0F),
                0x03 | 0x04 | 0x0F | 0x11 => self.next_stream_id(STREAM_ID_AUDIO, 0x1F),
                0x06 | 0x81 | 0x82 | 0x83 | 0x84 | 0x87 => STREAM_ID_PRIVATE_1,
                _ => STREAM_ID_PRIVATE_1,
            };
            let pmt_stream = PmtStream {
                stream_type: stream.stream_type,
                elementary_pid: stream.elementary_pid,
                stream_descriptors: stream.stream_descriptors.clone(),
            };
            self.streams
                .push(ElementaryStream::new(stream_id, pmt_stream));
        }

        self.psi_dirty = true;
        self.program_number = service.program_number;
        self.pmt_pid = service.pmt_pid;
        self.pmt_descriptors = service.program_descriptors.clone();
        self.pmt_packetizer = PsiPacketizer::new(self.pmt_pid);
    }

    /// Finds stream index by elementary PID
    pub fn stream_index(&self, elementary_pid: u16) -> Option<usize> {
        self.streams
            .iter()
            .position(|stream| stream.pmt_stream.elementary_pid == elementary_pid)
    }

    /// Pushes an ES frame for the given stream.
    ///
    /// - `stream_index` - stream index returned by [`stream_index`](Self::stream_index)
    /// - `frame` - ES frame data and metadata
    pub fn push_frame(&mut self, stream_index: usize, frame: MuxFrame) {
        if let Some(stream) = self.streams.get_mut(stream_index) {
            stream.push_frame(frame);
        }
    }

    /// Drains queued frames into TS packets written to `buf`.
    ///
    /// Returns the number of bytes written (always a multiple of 188).
    pub fn drain(&mut self, buf: &mut [u8]) -> usize {
        let (packets, _) = buf.as_chunks_mut::<PACKET_SIZE>();
        let mut written = 0;

        while written < packets.len() {
            match self.state {
                MultiplexerState::Idle => {
                    self.load_frames();

                    let Some((idx, meta)) = self.select_stream() else {
                        break;
                    };

                    if self.check_psi_state(&meta) {
                        self.prepare_psi_state(idx, &meta);
                    } else if self.check_pcr_state(&meta) {
                        self.prepare_pcr_state(idx, &meta);
                    } else {
                        self.state = MultiplexerState::EmitFrame(idx);
                    }
                }

                MultiplexerState::EmitPat => {
                    written += self.emit_pat(&mut packets[written ..]);
                }

                MultiplexerState::EmitPmt => {
                    written += self.emit_pmt(&mut packets[written ..]);
                }

                MultiplexerState::EmitPcr(timestamp) => {
                    written += self.emit_pcr(timestamp, &mut packets[written ..]);
                }

                MultiplexerState::EmitFrame(idx) => {
                    written += self.emit_stream(idx, &mut packets[written ..]);
                }
            }
        }

        written * PACKET_SIZE
    }

    fn check_psi_state(&self, meta: &ActiveFrame) -> bool {
        self.psi_dirty
            || meta.pending_key_psi
            || is_timestamp_delta_exceeded(self.last_psi_timestamp, meta.timestamp, PSI_INTERVAL)
    }

    fn prepare_psi_state(&mut self, idx: usize, meta: &ActiveFrame) {
        if self.psi_dirty {
            self.rebuild();
            self.psi_dirty = false;
        } else {
            self.pat_packetizer.reset();
            self.pmt_packetizer.reset();
        }

        self.last_psi_timestamp = meta.timestamp;
        self.streams[idx].active.as_mut().unwrap().pending_key_psi = false;

        self.state = MultiplexerState::EmitPat;
    }

    fn check_pcr_state(&self, meta: &ActiveFrame) -> bool {
        meta.pending_key_pcr
            || is_timestamp_delta_exceeded(self.last_pcr_timestamp, meta.timestamp, PCR_INTERVAL)
    }

    fn prepare_pcr_state(&mut self, idx: usize, meta: &ActiveFrame) {
        self.last_pcr_timestamp = meta.timestamp;
        self.streams[idx].active.as_mut().unwrap().pending_key_pcr = false;

        self.state = MultiplexerState::EmitPcr(meta.timestamp.unwrap());
    }

    fn load_frames(&mut self) {
        self.streams
            .iter_mut()
            .filter(|s| s.active.is_none())
            .for_each(|s| s.load_next_frame());
    }

    /// Selects the stream with the earliest timestamp
    fn select_stream(&self) -> Option<(usize, ActiveFrame)> {
        let mut result = None;
        let mut timestamp = None;

        for (i, stream) in self.streams.iter().enumerate() {
            if let Some(active) = stream.active {
                match (timestamp, active.timestamp) {
                    (None, Some(ts2)) => {
                        result = Some((i, active));
                        timestamp = Some(ts2);
                    }
                    (Some(ts1), Some(ts2)) if ts2.is_before(ts1) => {
                        result = Some((i, active));
                        timestamp = Some(ts2);
                    }
                    _ => {}
                }
            }
        }

        result
    }

    /// Rebuild PAT and PMT sections from current stream configuration. Should be
    /// called after adding streams or changing PMT parameters.
    fn rebuild(&mut self) {
        let pcr_pid = self
            .streams
            .first()
            .map(|s| s.pmt_stream.elementary_pid)
            .unwrap_or(0x1FFF);

        let pat_sections = PatBuilder::build(PatConfig {
            transport_stream_id: self.tsid,
            version: 0,
            programs: vec![PatProgram {
                program_number: self.program_number,
                pid: self.pmt_pid,
            }],
        });
        self.pat_packetizer.set_sections(pat_sections);

        let pmt_sections = PmtBuilder::build(PmtConfig {
            program_number: self.program_number,
            pcr_pid,
            version: 0,
            program_descriptors: self.pmt_descriptors.clone(),
            streams: self
                .streams
                .iter()
                .map(|stream| stream.pmt_stream.clone())
                .collect(),
        });
        self.pmt_packetizer.set_sections(pmt_sections);
    }

    fn next_stream_id(&self, base: u8, limit: u8) -> u8 {
        let max = base + limit;
        let count = self
            .streams
            .iter()
            .filter(|s| s.stream_id >= base && s.stream_id < max)
            .count();
        // TODO: handle overflow
        base + count as u8
    }

    /// Emit TS packets with PAT
    fn emit_pat(&mut self, packets: &mut [[u8; PACKET_SIZE]]) -> usize {
        let mut written = 0;

        while written < packets.len() {
            let packet = &mut packets[written];
            if self.pat_packetizer.next(packet) {
                written += 1;
            } else {
                self.state = MultiplexerState::EmitPmt;
                break;
            }
        }

        written
    }

    /// Emit TS packets with PMT
    fn emit_pmt(&mut self, packets: &mut [[u8; PACKET_SIZE]]) -> usize {
        let mut written = 0;

        while written < packets.len() {
            let packet = &mut packets[written];
            if self.pmt_packetizer.next(packet) {
                written += 1;
            } else {
                self.state = MultiplexerState::Idle;
                break;
            }
        }

        written
    }

    /// Emit TS packet with PCR
    fn emit_pcr(&mut self, timestamp: Timestamp, packets: &mut [[u8; PACKET_SIZE]]) -> usize {
        let pcr = timestamp.wrapping_sub(PCR_DELAY).value() * 300;

        if let Some(packet) = packets.get_mut(0) {
            self.streams[0].packetizer.build_pcr_packet(packet, pcr);
            self.state = MultiplexerState::Idle;

            1
        } else {
            0
        }
    }

    /// Emit TS packets for the active frame of a stream. Returns bytes written.
    fn emit_stream(&mut self, idx: usize, packets: &mut [[u8; PACKET_SIZE]]) -> usize {
        let stream = &mut self.streams[idx];
        if stream.active.is_none() {
            self.state = MultiplexerState::Idle;
            return 0;
        }

        let mut written = 0;
        while written < packets.len() {
            let packet = &mut packets[written];
            if stream.packetizer.next(packet) {
                written += 1;
            } else {
                stream.active = None;
                self.state = MultiplexerState::Idle;
                break;
            }
        }

        written
    }
}

fn is_timestamp_delta_exceeded(
    last: Option<Timestamp>,
    current: Option<Timestamp>,
    interval: u64,
) -> bool {
    match (last, current) {
        (_, None) => false,
        (Some(last), Some(current)) => current.wrapping_sub(last).value() >= interval,
        (None, Some(_)) => true,
    }
}
