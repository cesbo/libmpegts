use crate::ts::{
    PACKET_SIZE,
    TsPacketMut,
};

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

    /// Sets PTS and DTS values
    pub fn set_pts_dts(&mut self, pts: Option<u64>, dts: Option<u64>) {
        self.pts = pts;
        self.dts = dts;
    }

    /// Sets data alignment indicator
    pub fn set_data_alignment(&mut self) {
        self.data_alignment = true;
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

/// PES Packetizer - splits PES data into TS packets
///
/// Stores PES header and ES payload set via [`set_frame`](Self::set_frame)
/// and produces one TS packet per [`next`](Self::next) call
/// into a caller-provided buffer. Continuity counter persists across
/// [`set_frame`](Self::set_frame) calls.
///
/// # Example
/// ```
/// use libmpegts::pes::{PesHeader, PesPacketizer, STREAM_ID_VIDEO};
/// use libmpegts::ts::PACKET_SIZE;
///
/// let mut packetizer = PesPacketizer::new(101);
///
/// let mut header = PesHeader::new(STREAM_ID_VIDEO);
/// header.set_pts_dts(Some(90000), None);
/// let es_data = vec![0u8; 1000];
///
/// packetizer.set_frame(&header, es_data);
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
    data: Vec<u8>,
    offset: usize,
}

impl PesPacketizer {
    /// Creates a new PES packetizer for the given PID.
    pub fn new(pid: u16) -> Self {
        Self {
            pid,
            cc: 0,
            pes_header: [0u8; 32],
            pes_header_len: 0,
            data: Vec::new(),
            offset: 0,
        }
    }

    /// Sets PES header and ES payload for packetization.
    /// Resets position to the beginning.
    /// Continuity counter is preserved for CC continuity across frames.
    pub fn set_frame(&mut self, header: &PesHeader, data: Vec<u8>) {
        self.pes_header_len = header.write(&mut self.pes_header);
        self.data = data;
        self.offset = 0;
    }

    /// Writes the next TS packet into `packet`.
    /// Returns `true` if a packet was written, `false` when all data is exhausted.
    pub fn next(&mut self, packet: &mut [u8; PACKET_SIZE]) -> bool {
        let total = self.pes_header_len + self.data.len();
        if self.offset >= total {
            return false;
        }

        let remaining = total - self.offset;
        let stuffing = (PACKET_SIZE - 4).saturating_sub(remaining);

        let mut ts = TsPacketMut::from(&mut *packet);
        ts.set_sync();
        ts.set_pid(self.pid);
        ts.set_payload();
        ts.set_cc(self.cc);

        self.cc = (self.cc + 1) & 0x0F;

        if stuffing > 0 {
            ts.write_stuffing(stuffing);
        }

        let payload = if self.offset == 0 {
            // First packet of PES
            ts.set_pusi();
            let payload = ts.payload_mut().unwrap();
            self.offset = self.pes_header_len;
            payload[.. self.offset].copy_from_slice(&self.pes_header[.. self.offset]);
            &mut payload[self.offset ..]
        } else {
            // Continuation packet
            ts.payload_mut().unwrap()
        };

        let available = payload.len();
        let data_offset = self.offset - self.pes_header_len;
        let remaining = self.data.len() - data_offset;
        let to_copy = available.min(remaining);
        payload[.. to_copy].copy_from_slice(&self.data[data_offset .. data_offset + to_copy]);

        self.offset += to_copy;

        true
    }
}
