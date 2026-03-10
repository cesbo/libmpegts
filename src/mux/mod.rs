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
        PmtBuilder,
        PsiPacketizer,
    },
    ts::PACKET_SIZE,
};

const PCR_DELAY: Timestamp = Timestamp::new(700 * 90); // 700ms delay
const PCR_INTERVAL: u64 = 40 * 90; // 40ms in 90kHz ticks
const PSI_INTERVAL: u64 = 500 * 90; // 500ms in 90kHz ticks

/// Queued ES frame waiting to be packetized.
pub struct MuxFrame {
    pts_dts: Option<PtsDts>,
    is_key_frame: bool,
    data: Vec<u8>,
}

impl MuxFrame {
    /// Creates a new MuxFrame with given parameters
    ///
    /// - `data` - owned ES frame payload
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            pts_dts: None,
            is_key_frame: false,
            data,
        }
    }

    /// Sets the DTS
    ///
    /// - `pts` - Presentation Timestamp (90 kHz clock)
    /// - `dts` - Decoding Timestamp (90 kHz clock)
    pub fn with_pts_dts(mut self, pts_dts: impl Into<PtsDts>) -> Self {
        self.pts_dts = Some(pts_dts.into());
        self
    }

    /// Marks this frame as a key frame (for video) or access unit start (for audio).
    pub fn with_key_frame(mut self, value: bool) -> Self {
        self.is_key_frame = value;
        self
    }

    /// Frame DTS (or PTS if no DTS)
    fn timestamp(&self) -> Option<Timestamp> {
        self.pts_dts.map(|ts| ts.timestamp())
    }
}

/// Per-stream state inside the multiplexer.
pub struct MuxStream {
    stream_type: u8,
    pid: u16,
    descriptors: Option<Vec<u8>>,

    /// DTS (or PTS if no DTS) of the current frame, if any.
    current_timestamp: Option<Timestamp>,

    /// Assigned stream_id for PES headers (e.g. 0xE0 for video, 0xC0 for audio)
    stream_id: u8,
    packetizer: PesPacketizer,
    pending: VecDeque<MuxFrame>,
    draining: bool,
    pending_key_psi: bool,
    pending_key_pcr: bool,
}

impl MuxStream {
    /// Creates a new stream with the given type and PID.
    ///
    /// - `stream_type` - MPEG-TS stream type (e.g. 0x1B for H.264, 0x0F for AAC)
    /// - `pid` - PID assigned to this stream (must be unique and >= 0x20 and < 0x1FFF)
    pub fn new(stream_type: u8, pid: u16) -> Self {
        Self {
            stream_type,
            pid,
            descriptors: None,

            current_timestamp: None,
            stream_id: 0,
            packetizer: PesPacketizer::new(pid),
            pending: VecDeque::new(),
            draining: false,
            pending_key_psi: false,
            pending_key_pcr: false,
        }
    }

    /// Sets the ES-level descriptors for this stream (to be included in PMT).
    pub fn set_descriptors(&mut self, descriptors: Option<&[u8]>) {
        self.descriptors = descriptors.map(|d| d.to_vec());
    }

    fn push_frame(&mut self, frame: MuxFrame) {
        self.pending.push_back(frame);
    }

    /// Feed the next queued frame into the packetizer.
    /// Returns `true` if a frame was loaded, `false` if the queue is empty.
    fn load_next_frame(&mut self) -> bool {
        if let Some(frame) = self.pending.pop_front() {
            self.current_timestamp = frame.timestamp();

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
            self.draining = true;
            self.pending_key_psi = frame.is_key_frame;
            self.pending_key_pcr = frame.is_key_frame;

            true
        } else {
            false
        }
    }
}

#[derive(Clone, Copy)]
enum MuxState {
    Idle,
    EmitPat,
    EmitPmt,
    EmitPcr,
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

    pnr: u16,
    pmt_pid: u16,
    pmt_descriptors: Option<Vec<u8>>,
    pmt_packetizer: PsiPacketizer,

    state: MuxState,
    psi_dirty: bool,

    /// Registered elementary streams.
    streams: Vec<MuxStream>,

    last_pcr_timestamp: Option<Timestamp>,
    last_psi_timestamp: Option<Timestamp>,
}

impl Multiplexer {
    /// Creates a new single-program multiplexer.
    pub fn new(tsid: u16) -> Self {
        Self {
            tsid,
            pat_packetizer: PsiPacketizer::new(PAT_PID),

            pnr: 1,
            pmt_pid: 256,
            pmt_descriptors: None,
            pmt_packetizer: PsiPacketizer::new(256),

            state: MuxState::Idle,
            psi_dirty: true,

            streams: Vec::new(),
            last_pcr_timestamp: None,
            last_psi_timestamp: None,
        }
    }

    /// Sets service parameters for the program
    ///
    /// - `pnr` - program number (must be unique and > 0)
    /// - `pid` - PID for PMT (must be unique and >= 0x20 and < 0x1FFF)
    ///
    /// TODO:
    /// - add_service() to register multiple programs
    /// - each program can have multiple streams
    /// - same stream can be in multiple programs
    pub fn set_service(&mut self, pnr: u16, pid: u16, descriptors: Option<&[u8]>) {
        self.psi_dirty = true;
        self.pnr = pnr;
        self.pmt_pid = pid;
        self.pmt_descriptors = descriptors.map(|d| d.to_vec());
        self.pmt_packetizer = PsiPacketizer::new(pid);
    }

    /// Registers a new elementary stream and returns its stream index.
    ///
    /// The first registered stream becomes the PCR PID.
    ///
    /// - `stream_type` - MPEG-TS stream type (e.g. 0x1B for H.264)
    /// - `pid` - PID to assign to this stream (must be unique and >= 0x20 and < 0x1FFF)
    /// - `descriptors` - raw ES-level descriptor bytes for PMT
    pub fn add_stream(&mut self, mut stream: MuxStream) -> usize {
        stream.stream_id = match stream.stream_type {
            0x02 | 0x1B | 0x24 | 0x33 => self.next_stream_id(STREAM_ID_VIDEO, 0x0F),
            0x03 | 0x04 | 0x0F | 0x11 => self.next_stream_id(STREAM_ID_AUDIO, 0x1F),
            0x06 | 0x81 | 0x82 | 0x83 | 0x84 | 0x87 => STREAM_ID_PRIVATE_1,
            _ => STREAM_ID_PRIVATE_1,
        };

        self.psi_dirty = true;

        self.streams.push(stream);
        self.streams.len() - 1
    }

    /// Pushes an ES frame for the given stream.
    ///
    /// - `stream_id` - stream index returned by [`add_stream`](Self::add_stream)
    /// - `frame` - ES frame data and metadata
    pub fn push_frame(&mut self, stream_id: usize, frame: MuxFrame) {
        if let Some(stream) = self.streams.get_mut(stream_id) {
            stream.push_frame(frame);
        }
    }

    /// Drains queued frames into TS packets written to `buf`.
    ///
    /// Returns the number of bytes written (always a multiple of 188).
    pub fn drain(&mut self, buf: &mut [u8]) -> usize {
        if self.streams.is_empty() {
            return 0;
        }

        let (packets, _) = buf.as_chunks_mut::<PACKET_SIZE>();
        let mut written = 0;

        while written < packets.len() {
            match self.state {
                MuxState::Idle => {
                    let Some(state) = self.select_state() else {
                        break;
                    };
                    self.state = state;
                }

                MuxState::EmitPat => {
                    let packet = &mut packets[written];
                    if self.pat_packetizer.next(packet) {
                        written += 1;
                    } else {
                        self.state = MuxState::EmitPmt;
                    }
                }

                MuxState::EmitPmt => {
                    let packet = &mut packets[written];
                    if self.pmt_packetizer.next(packet) {
                        written += 1;
                    } else {
                        self.state = MuxState::Idle;
                    }
                }

                MuxState::EmitPcr => {
                    let timestamp = self.last_pcr_timestamp.unwrap(); // TODO: without option
                    let pcr = timestamp.wrapping_sub(PCR_DELAY).value() * 300;
                    let packet = &mut packets[written];
                    self.streams[0].packetizer.build_pcr_packet(packet, pcr);
                    written += 1;
                    self.state = MuxState::Idle;
                }

                MuxState::EmitFrame(idx) => {
                    let n = self.emit_stream(idx, &mut packets[written ..]);
                    if n == 0 {
                        self.streams[idx].draining = false;
                        self.state = MuxState::Idle;
                    } else {
                        written += n;
                        if !self.streams[idx].draining {
                            self.state = MuxState::Idle;
                        }
                    }
                }
            }
        }

        written * PACKET_SIZE
    }

    fn select_state(&mut self) -> Option<MuxState> {
        let idx = self.select_stream()?;
        let timestamp = self.streams[idx].current_timestamp;

        if self.psi_dirty
            || self.streams[idx].pending_key_psi
            || is_timestamp_delta_exceeded(self.last_psi_timestamp, timestamp, PSI_INTERVAL)
        {
            if self.psi_dirty {
                self.rebuild();
                self.psi_dirty = false;
            } else {
                self.pat_packetizer.reset();
                self.pmt_packetizer.reset();
            }

            self.streams[idx].pending_key_psi = false;
            self.last_psi_timestamp = timestamp;
            return Some(MuxState::EmitPat);
        }

        if self.streams[idx].pending_key_pcr
            || is_timestamp_delta_exceeded(self.last_pcr_timestamp, timestamp, PCR_INTERVAL)
        {
            self.streams[idx].pending_key_pcr = false;
            self.last_pcr_timestamp = timestamp;
            return Some(MuxState::EmitPcr);
        }

        Some(MuxState::EmitFrame(idx))
    }

    /// Load missing active frames and select the stream with the earliest timestamp.
    fn select_stream(&mut self) -> Option<usize> {
        let mut idx = None;
        let mut timestamp = None;

        for (i, stream) in self.streams.iter_mut().enumerate() {
            if !stream.draining && !stream.pending.is_empty() {
                stream.load_next_frame();
            }

            if stream.draining {
                match (timestamp, stream.current_timestamp) {
                    (None, Some(ts2)) => {
                        idx = Some(i);
                        timestamp = Some(ts2);
                    }
                    (Some(ts1), Some(ts2)) if ts2.is_before(ts1) => {
                        idx = Some(i);
                        timestamp = Some(ts2);
                    }
                    _ => {}
                }
            }
        }

        idx
    }

    /// Rebuild PAT and PMT sections from current stream configuration. Should be
    /// called after adding streams or changing PMT parameters.
    fn rebuild(&mut self) {
        let pcr_pid = self.streams.first().map(|s| s.pid).unwrap_or(0x1FFF);

        let mut pat_builder = PatBuilder::new(self.tsid);
        pat_builder.push(self.pnr, self.pmt_pid);
        let pat_sections = pat_builder.finalize();
        self.pat_packetizer.set_sections(pat_sections);

        let mut pmt_builder = PmtBuilder::new(self.pnr, pcr_pid);
        pmt_builder.set_descriptors(self.pmt_descriptors.as_deref());
        for stream in &self.streams {
            pmt_builder.push(
                stream.stream_type,
                stream.pid,
                stream.descriptors.as_deref(),
            );
        }

        let pmt_sections = pmt_builder.finalize();
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

    /// Emit TS packets for the active frame of a stream. Returns bytes written.
    fn emit_stream(&mut self, idx: usize, packets: &mut [[u8; PACKET_SIZE]]) -> usize {
        let stream = &mut self.streams[idx];
        if !stream.draining {
            return 0;
        }

        let mut written = 0;
        while written < packets.len() {
            let packet = &mut packets[written];
            if stream.packetizer.next(packet) {
                written += 1;
            } else {
                stream.draining = false;
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
