mod error;
mod packetizer;

pub use error::*;
pub use packetizer::PesPacketizer;

/// PTS - Presentation Timestamp
/// 90clocks = 1ms
pub const PTS_MAX: u64 = PTS_NONE - 1;
pub const PTS_NONE: u64 = 1 << 33;
pub const PTS_CLOCK_MS: u64 = 90;

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
    /// Presentation Timestamp (90kHz clock)
    pts: Option<u64>,
    /// Decoding Timestamp (90kHz clock), only valid with PTS
    dts: Option<u64>,
    /// Data alignment indicator
    data_alignment: bool,
}

impl PesHeader {
    /// Creates a new PES header with given stream_id
    pub fn new(stream_id: u8) -> Self {
        Self {
            stream_id,
            pts: None,
            dts: None,
            data_alignment: false,
        }
    }

    /// Sets PTS value
    pub fn with_pts(mut self, pts: u64) -> Self {
        self.pts = Some(pts);
        self
    }

    /// Sets PTS and DTS values
    pub fn with_pts_dts(mut self, pts: u64, dts: u64) -> Self {
        self.pts = Some(pts);
        self.dts = Some(dts);
        self
    }

    /// Sets data alignment indicator
    pub fn with_data_alignment(mut self) -> Self {
        self.data_alignment = true;
        self
    }

    /// Returns the size of the PES header in bytes
    pub fn size(&self) -> usize {
        // packet_start_code_prefix (3) + stream_id (1) + pes_packet_length (2) = 6
        // + optional_pes_header (3 minimum: flags + header_data_length)
        let base = 6 + 3;

        let pts_dts_size = match (self.pts, self.dts) {
            (Some(_), Some(_)) => 10, // PTS (5) + DTS (5)
            (Some(_), None) => 5,     // PTS only
            _ => 0,
        };

        base + pts_dts_size
    }

    /// Writes PES header to buffer, returns number of bytes written
    ///
    /// # Panics
    /// Panics if buffer is too small
    pub fn write(&self, buf: &mut [u8]) -> usize {
        let size = self.size();
        assert!(buf.len() >= size, "buffer too small for PES header");

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
        let pts_dts_flags = match (self.pts, self.dts) {
            (Some(_), Some(_)) => 0b11, // PTS and DTS
            (Some(_), None) => 0b10,    // PTS only
            _ => 0b00,
        };
        buf[7] = pts_dts_flags << 6;

        // Byte 8: PES header data length
        let header_data_length = match (self.pts, self.dts) {
            (Some(_), Some(_)) => 10,
            (Some(_), None) => 5,
            _ => 0,
        };
        buf[8] = header_data_length;

        let mut offset = 9;

        // Write PTS
        if let Some(pts) = self.pts {
            let marker = if self.dts.is_some() { 0b0011 } else { 0b0010 };
            Self::write_timestamp(&mut buf[offset ..], pts, marker);
            offset += 5;
        }

        // Write DTS
        if let Some(dts) = self.dts {
            Self::write_timestamp(&mut buf[offset ..], dts, 0b0001);
            offset += 5;
        }

        offset
    }

    /// Writes 33-bit timestamp in PES format (5 bytes)
    fn write_timestamp(buf: &mut [u8], ts: u64, marker: u8) {
        // PTS/DTS format (5 bytes, 40 bits total):
        let ts = ts & PTS_MAX;

        buf[0] = (marker << 4) | ((ts >> 29) & 0x0E) as u8 | 0x01;
        buf[1] = (ts >> 22) as u8;
        buf[2] = ((ts >> 14) & 0xFE) as u8 | 0x01;
        buf[3] = (ts >> 7) as u8;
        buf[4] = ((ts << 1) & 0xFE) as u8 | 0x01;
    }
}
