mod timestamp;

pub use timestamp::{
    PtsDts,
    Timestamp,
};

use crate::ts::{
    PACKET_SIZE,
    TsPacketMut,
};

/// Stream ID constants
pub const STREAM_ID_VIDEO: u8 = 0xE0; // First video stream
pub const STREAM_ID_AUDIO: u8 = 0xC0; // First audio stream
pub const STREAM_ID_PRIVATE_1: u8 = 0xBD; // AC3, DTS, etc.
pub const STREAM_ID_PRIVATE_2: u8 = 0xBF;

/// PES header structure for building PES packets
#[derive(Debug, Clone)]
pub struct PesHeader {
    /// Stream ID (0xE0 for video, 0xC0 for audio, etc.)
    stream_id: u8,
    /// Presentation Timestamp and Decoding Timestamp (90kHz clock)
    pts_dts: Option<PtsDts>,
    /// Data alignment indicator
    data_alignment: bool,
}

impl PesHeader {
    /// Creates a new PES header with given stream_id
    pub fn new(stream_id: u8) -> Self {
        Self {
            stream_id,
            pts_dts: None,
            data_alignment: false,
        }
    }

    /// Sets PTS and DTS values
    pub fn with_pts_dts(mut self, pts_dts: impl Into<PtsDts>) -> Self {
        self.pts_dts = Some(pts_dts.into());
        self
    }

    /// Sets data alignment indicator
    pub fn with_data_alignment(mut self, value: bool) -> Self {
        self.data_alignment = value;
        self
    }

    /// Writes PES header to buffer, returns number of bytes written
    ///
    /// # Panics
    /// Panics if buffer is too small
    pub fn write(&self, buf: &mut [u8]) -> usize {
        debug_assert!(buf.len() >= 32, "buffer too small for PES header");

        // Packet start code prefix: 0x00 0x00 0x01
        buf[0] = 0x00;
        buf[1] = 0x00;
        buf[2] = 0x01;

        // Stream ID
        buf[3] = self.stream_id;

        // PES packet length: 0 for unbounded (video)
        buf[4] = 0x00;
        buf[5] = 0x00;

        // Optional PES header
        // Byte 6: '10' + scrambling(2) + priority(1) + data_alignment(1) + copyright(1) + original(1)
        let flags_1 = 0x80 | if self.data_alignment { 0x04 } else { 0x00 };
        buf[6] = flags_1;

        // Byte 7: pts_dts_flags(2) + escr(1) + es_rate(1) + dsm_trick(1) + additional_copy(1) + crc(1) + ext(1)
        buf[7] = 0x00;

        // Byte 8: PES header data length
        buf[8] = 0x00;
        let mut offset = 9;

        if let Some(pts_dts) = self.pts_dts {
            let pts = pts_dts.pts;

            if let Some(dts) = pts_dts.dts {
                buf[7] |= 0b1100_0000; // PTS and DTS
                buf[8] += 10; // 5 bytes for PTS + 5 bytes for DTS

                offset += pts.write(&mut buf[offset ..], 0b0011);
                offset += dts.write(&mut buf[offset ..], 0b0001);
            } else {
                buf[7] |= 0b1000_0000; // PTS only
                buf[8] += 5; // 5 bytes for PTS

                offset += pts.write(&mut buf[offset ..], 0b0010);
            }
        }

        offset
    }
}

/// Elementary stream frame with PES header, payload, and TS-level metadata.
pub struct EsFrame {
    pub header: PesHeader,
    pub payload: Vec<u8>,

    /// First TS packet with `random_access_indicator` when `true`
    pub rai: bool,
}

/// PES Packetizer - splits PES data into TS packets
///
/// Stores EsFrame set via [`set_frame`](Self::set_frame)
/// and produces one TS packet per [`next`](Self::next) call
/// into a caller-provided buffer.
/// Continuity counter persists across [`set_frame`](Self::set_frame) calls.
///
/// # Example
/// ```
/// use libmpegts::pes::{EsFrame, PesHeader, PesPacketizer, STREAM_ID_VIDEO, PtsDts};
/// use libmpegts::ts::PACKET_SIZE;
///
/// let mut packetizer = PesPacketizer::new(101);
///
/// let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(PtsDts::new(90000));
/// let payload = vec![0u8; 1000];
/// let frame = EsFrame { header, payload, rai: false };
/// packetizer.set_frame(frame);
///
/// let mut packet = [0u8; PACKET_SIZE];
/// while packetizer.next(&mut packet) {
///     // process packet
/// }
/// ```
pub struct PesPacketizer {
    pid: u16,
    cc: u8,

    pes_header: [u8; 32],
    pes_header_len: usize,
    offset: usize,

    frame: Option<EsFrame>,
}

impl PesPacketizer {
    /// Creates a new PES packetizer for the given PID.
    pub fn new(pid: u16) -> Self {
        Self {
            pid,
            cc: 0,

            pes_header: [0u8; 32],
            pes_header_len: 0,
            offset: 0,

            frame: None,
        }
    }

    /// Sets PES header and ES payload for packetization.
    /// Resets position to the beginning.
    /// Continuity counter is preserved for CC continuity across frames.
    pub fn set_frame(&mut self, frame: EsFrame) {
        self.pes_header_len = frame.header.write(&mut self.pes_header);
        self.offset = 0;
        self.frame = Some(frame);
    }

    pub fn build_pcr_packet(&mut self, packet: &mut [u8; PACKET_SIZE], pcr: u64) {
        let mut ts = TsPacketMut::from(packet);
        ts.init(self.pid, self.cc.wrapping_sub(1) & 0x0F);
        ts.set_adaptation_field(PACKET_SIZE - 4);
        ts.set_pcr(pcr);
    }

    /// Writes the next TS packet into `packet`.
    /// Returns `true` if a packet was written, `false` when all data is exhausted.
    pub fn next(&mut self, packet: &mut [u8; PACKET_SIZE]) -> bool {
        let Some(frame) = &self.frame else {
            return false;
        };

        let is_first = self.offset == 0;

        let total = self.pes_header_len + frame.payload.len();
        let remaining = total - self.offset;

        // Calculate AF size for RAI and/or stuffing
        let mut af_size: usize = 0;
        let max_payload = PACKET_SIZE - 4;

        if is_first && frame.rai {
            af_size = 2; // length byte + flags byte
        }

        let capacity = max_payload - af_size;

        // Stuffing
        if remaining < capacity {
            let stuffing = capacity - remaining;
            af_size += stuffing;
        }

        // Build TS packet
        let mut ts = TsPacketMut::from(&mut *packet);
        ts.init(self.pid, self.cc);
        ts.set_payload();
        self.cc = (self.cc + 1) & 0x0F;

        if af_size > 0 {
            ts.set_adaptation_field(af_size);
        }

        if is_first {
            ts.set_pusi();

            if frame.rai {
                ts.set_rai();
            }
        }

        let ts_payload = ts.payload_mut().unwrap();
        let mut pos = 0;

        // PES header
        if self.offset < self.pes_header_len {
            let n = (self.pes_header_len - self.offset).min(ts_payload.len());
            ts_payload[.. n].copy_from_slice(&self.pes_header[self.offset .. self.offset + n]);
            self.offset += n;
            pos = n;
        }

        // ES payload
        if self.offset >= self.pes_header_len {
            let data_offset = self.offset - self.pes_header_len;
            let space = ts_payload.len() - pos;
            let n = (frame.payload.len() - data_offset).min(space);
            if n > 0 {
                ts_payload[pos .. pos + n]
                    .copy_from_slice(&frame.payload[data_offset .. data_offset + n]);
                self.offset += n;
            }
        }

        if self.offset >= total {
            self.frame = None;
        }

        true
    }
}
