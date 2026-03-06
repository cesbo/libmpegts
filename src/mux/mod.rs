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

const PCR_DELAY: u64 = 700 * 90; // 700ms delay

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
    pub fn with_pts_dts(mut self, pts_dts: PtsDts) -> Self {
        self.pts_dts = Some(pts_dts);
        self
    }

    /// Marks this frame as a key frame (for video) or access unit start (for audio).
    pub fn with_key_frame(mut self, value: bool) -> Self {
        self.is_key_frame = value;
        self
    }

    /// Frame DTS (or PTS if no DTS)
    fn timestamp(&self) -> Option<Timestamp> {
        self.pts_dts.map(|ts| ts.dts.unwrap_or(ts.pts))
    }
}

/// Per-stream state inside the multiplexer.
pub struct MuxStream {
    stream_type: u8,
    pid: u16,
    descriptors: Option<Vec<u8>>,

    /// DTS (or PTS if no DTS) of the front frame, if any
    current_timestamp: Option<Timestamp>,

    /// Assigned stream_id for PES headers (e.g. 0xE0 for video, 0xC0 for audio)
    stream_id: u8,
    packetizer: PesPacketizer,
    pending: VecDeque<MuxFrame>,
    draining: bool,
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
            true
        } else {
            false
        }
    }
}

enum PsiEmitState {
    Idle,
    Start,
    EmitPat,
    EmitPmt(usize),
}

/// Single-program (SPTS) VBR multiplexer.
///
/// Accepts ES frames via [`push_frame`](Self::push_frame) and produces
/// interleaved MPEG-TS packets with auto-generated PAT, PMT, and PCR
/// via [`drain`](Self::drain).
///
/// # Example
///
/// ```
/// use libmpegts::mux::{Multiplexer, MuxFrame, MuxStream};
/// use libmpegts::pes::PtsDts;
///
/// let mut mux = Multiplexer::new(1);
/// let video = mux.add_stream(MuxStream::new(0x1B, 101));
/// let audio = mux.add_stream(MuxStream::new(0x0F, 102));
///
/// // Push a video key frame
/// let frame = MuxFrame::new(vec![0u8; 50000])
///   .with_key_frame(true)
///   .with_pts_dts(PtsDts::new(90000).with_dts(90000));
/// mux.push_frame(video, frame);
/// // Push an audio frame
/// let frame = MuxFrame::new(vec![0u8; 1024])
///   .with_pts_dts(PtsDts::new(90000));
/// mux.push_frame(audio, frame);
///
/// let mut buf = [0u8; 188 * 1000];
/// let n = mux.drain(&mut buf);
/// assert!(n > 0);
/// assert_eq!(n % 188, 0);
/// ```
pub struct Multiplexer {
    tsid: u16,
    pat_packetizer: PsiPacketizer,

    pnr: u16,
    pmt_pid: u16,
    pmt_descriptors: Option<Vec<u8>>,
    pmt_packetizer: PsiPacketizer,

    psi_state: PsiEmitState,
    psi_dirty: bool,

    /// Registered elementary streams.
    streams: Vec<MuxStream>,

    /// Scheduler for VBR interleaving.
    // scheduler: Scheduler,
    current_timestamp: Option<Timestamp>,
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

            // Rebuild PSI tables and emit on the first drain() call
            psi_state: PsiEmitState::Start,
            psi_dirty: true,

            streams: Vec::new(),
            current_timestamp: None,
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
        // Assign unique stream_id
        stream.stream_id = match stream.stream_type {
            // H.262, H.264, H.265, H.266
            0x02 | 0x1B | 0x24 | 0x33 => self.next_stream_id(STREAM_ID_VIDEO, 0x0F),
            // MPEG-1/2 Audio, AAC
            0x03 | 0x04 | 0x0F | 0x11 => self.next_stream_id(STREAM_ID_AUDIO, 0x1F),
            // AC-3, E-AC-3, DTS, etc.
            // TODO: implement stream_identifier_descriptor (0x52)
            0x06 | 0x81 | 0x82 | 0x83 | 0x84 | 0x87 => STREAM_ID_PRIVATE_1,
            // Other types
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

        // Update current timeline from the earliest pending frame
        self.update_timestamp();

        let capacity = buf.len() / PACKET_SIZE;
        let mut written = 0;
        let buf = &mut buf[.. capacity * PACKET_SIZE];

        // Emit pending PSI packets
        if !matches!(self.psi_state, PsiEmitState::Idle) {
            let n = self.emit_psi(buf);
            written += n;
        }

        // while written < capacity {
        // TODO: implement a scheduler
        // It should send PSI for every new key frame, PCR every ~40 ms, and interleave ES packets.
        // }

        written * PACKET_SIZE
    }

    /// Rebuild PAT and PMT sections from current stream configuration. Should be
    /// called after adding streams or changing PMT parameters.
    fn rebuild(&mut self) {
        let pcr_pid = self.streams.first().map(|s| s.pid).unwrap_or(0x1FFF);

        // Build PAT
        let mut pat_builder = PatBuilder::new(self.tsid);
        pat_builder.push(self.pnr, self.pmt_pid);

        let pat_sections = pat_builder.finalize();
        self.pat_packetizer.set_sections(pat_sections);

        // Build PMT
        let mut pmt_builder = PmtBuilder::new(self.pnr, pcr_pid);
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

    /// Update current PTS from the earliest pending frame across all streams.
    fn update_timestamp(&mut self) {
        self.current_timestamp = self
            .streams
            .iter()
            .filter_map(|x| x.current_timestamp)
            .min();
    }

    /// Emit PAT + PMT packets. Returns number of packets written.
    ///
    /// Writes as many packets as possible into `buf` (must be aligned to [`PACKET_SIZE`]).
    /// State machine: `Start` → `EmitPat` → `EmitPmt` → `Idle`.
    fn emit_psi(&mut self, buf: &mut [u8]) -> usize {
        let mut offset = 0;

        while offset < buf.len() {
            match self.psi_state {
                PsiEmitState::Idle => break,

                PsiEmitState::Start => {
                    if self.psi_dirty {
                        self.rebuild();
                        self.psi_dirty = false;
                    } else {
                        self.pat_packetizer.reset();
                        self.pmt_packetizer.reset();
                    }
                    self.psi_state = PsiEmitState::EmitPat;
                }

                PsiEmitState::EmitPat => {
                    let packet =
                        unsafe { &mut *buf.as_mut_ptr().add(offset).cast::<[u8; PACKET_SIZE]>() };
                    if self.pat_packetizer.next(packet) {
                        offset += PACKET_SIZE;
                    } else {
                        self.psi_state = PsiEmitState::EmitPmt(0);
                    }
                }

                PsiEmitState::EmitPmt(_service_idx) => {
                    let packet =
                        unsafe { &mut *buf.as_mut_ptr().add(offset).cast::<[u8; PACKET_SIZE]>() };
                    if self.pmt_packetizer.next(packet) {
                        offset += PACKET_SIZE;
                    } else {
                        // TODO: for MPTS iterate over services
                        self.psi_state = PsiEmitState::Idle;
                    }
                }
            }
        }

        offset / PACKET_SIZE
    }

    /// Emit a PCR-only packet.
    fn emit_pcr(&mut self, buf: &mut [u8]) {
        // Convert PTS (90 kHz) to PCR (27 MHz): pcr = pts * 300
        let Some(current_timestamp) = self.current_timestamp else {
            return;
        };

        let pcr_timestamp = current_timestamp.wrapping_sub(PCR_DELAY);
        let pcr = pcr_timestamp.value() * 300;
        let packet = unsafe { &mut *buf.as_mut_ptr().cast::<[u8; PACKET_SIZE]>() };
        self.streams[0].packetizer.build_pcr_packet(packet, pcr);
    }

    /// Emit TS packets for a stream. Returns number of packets written.
    fn emit_stream(&mut self, idx: usize, buf: &mut [u8], max_packets: usize) -> usize {
        let stream = &mut self.streams[idx];
        let mut count = 0;

        // If not currently draining a frame, load the next one
        if !stream.draining && !stream.load_next_frame() {
            return 0;
        }

        // Emit TS packets from the PES packetizer
        while count < max_packets {
            let packet = unsafe {
                &mut *buf
                    .as_mut_ptr()
                    .add(count * PACKET_SIZE)
                    .cast::<[u8; PACKET_SIZE]>()
            };
            if stream.packetizer.next(packet) {
                count += 1;
            } else {
                stream.draining = false;
                break;
            }
        }

        count
    }
}
