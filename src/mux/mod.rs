use std::collections::VecDeque;

use crate::{
    pes::{
        PTS_NONE,
        PesHeader,
        PesPacketizer,
        STREAM_ID_AUDIO,
        STREAM_ID_PRIVATE_1,
        STREAM_ID_VIDEO,
    },
    psi::{
        PAT_PID,
        PatBuilder,
        PmtBuilder,
        PsiPacketizer,
    },
    ts::PACKET_SIZE,
};

/// Queued ES frame waiting to be packetized.
pub struct MuxFrame {
    pts: u64,
    dts: Option<u64>,
    is_key_frame: bool,
    data: Vec<u8>,
}

impl MuxFrame {
    /// Creates a new MuxFrame with given parameters
    ///
    /// - `pts` - Presentation Timestamp (90 kHz clock)
    /// - `data` - owned ES frame payload
    pub fn new(pts: u64, data: Vec<u8>) -> Self {
        Self {
            pts,
            dts: None,
            is_key_frame: false,
            data,
        }
    }

    /// Sets the DTS
    ///
    /// - `dts` - Decoding Timestamp (90 kHz clock)
    pub fn with_dts(mut self, dts: u64) -> Self {
        self.dts = Some(dts);
        self
    }

    /// Marks this frame as a key frame (for video) or access unit start (for audio).
    pub fn with_key_frame(mut self) -> Self {
        self.is_key_frame = true;
        self
    }
}

/// Per-stream state inside the multiplexer.
pub struct MuxStream {
    stream_type: u8,
    pid: u16,
    descriptors: Option<Vec<u8>>,

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

    /// Returns the DTS (or PTS if no DTS) of the front frame, if any.
    fn head_dts(&self) -> Option<u64> {
        self.pending.front().map(|f| f.dts.unwrap_or(f.pts))
    }

    /// Feed the next queued frame into the packetizer.
    /// Returns `true` if a frame was loaded, `false` if the queue is empty.
    fn load_next_frame(&mut self) -> bool {
        if let Some(frame) = self.pending.pop_front() {
            let mut header = PesHeader::new(self.stream_id);
            header.set_pts_dts(Some(frame.pts), frame.dts);
            if frame.is_key_frame {
                header.set_data_alignment();
            }
            self.packetizer.set_frame(&header, frame.data);
            self.draining = true;
            true
        } else {
            false
        }
    }
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
/// use libmpegts::mux::{Multiplexer, MuxStream};
///
/// let mut mux = Multiplexer::new();
/// let video = mux.add_stream(MuxStream::new(0x1B, 101));
/// let audio = mux.add_stream(MuxStream::new(0x0F, 102));
///
/// // Push a video key frame
/// mux.push_frame(video, 90000, Some(90000), true, vec![0u8; 50000]);
/// // Push an audio frame
/// mux.push_frame(audio, 90000, None, false, vec![0u8; 1024]);
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

    psi_dirty: bool,

    /// Registered elementary streams.
    streams: Vec<MuxStream>,

    /// Scheduler for VBR interleaving.
    // scheduler: Scheduler,
    current_pts: u64,
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

            psi_dirty: true,

            streams: Vec::new(),
            current_pts: 0,
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
        debug_assert!(stream_id < self.streams.len());
        self.streams[stream_id].pending.push_back(frame);
    }

    /// Drains queued frames into TS packets written to `buf`.
    ///
    /// Returns the number of bytes written (always a multiple of 188).
    pub fn drain(&mut self, buf: &mut [u8]) -> usize {
        if self.streams.is_empty() {
            return 0;
        }

        // Update current timeline from the earliest pending frame
        self.update_current_pts();

        let _capacity = buf.len() / PACKET_SIZE;
        let written = 0;

        // while written < capacity {
        // TODO: implement a scheduler
        // It should send PSI for every new key frame, PCR every ~40 ms, and interleave ES packets.
        // If psi_dirty then rebuild
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
    fn update_current_pts(&mut self) {
        let mut earliest = PTS_NONE;
        for stream in &self.streams {
            if let Some(dts) = stream.head_dts()
                && dts < earliest
            {
                earliest = dts;
            }
        }
        if earliest != PTS_NONE {
            self.current_pts = earliest;
        }
    }

    /// Emit PAT + PMT packets. Returns number of packets written.
    fn emit_psi(&mut self, buf: &mut [u8]) -> usize {
        // TODO: emit PAT and PMT
        // check current status:
        // - 0 - nothing to send
        // - 1 - PAT sending, when packetizer is finished, start PMT
        // - 2 - PMT sending, when packetizer is finished, set status to 0

        0
    }

    /// Emit a PCR-only packet.
    fn emit_pcr(&mut self, buf: &mut [u8]) {
        // Convert PTS (90 kHz) to PCR (27 MHz): pcr = pts * 300
        let pcr = self.current_pts.wrapping_mul(300);

        let packet: &mut [u8; PACKET_SIZE] = (&mut buf[.. PACKET_SIZE]).try_into().unwrap();
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
            let packet: &mut [u8; PACKET_SIZE] = (&mut buf
                [count * PACKET_SIZE .. (count + 1) * PACKET_SIZE])
                .try_into()
                .unwrap();
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
