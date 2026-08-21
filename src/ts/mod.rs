use std::fmt;

use crate::pcr::PCR_NONE;

pub const SYNC_BYTE: u8 = 0x47;
pub const PID_NONE: u16 = 8192;
pub const PID_NULL: u16 = PID_NONE - 1;
pub const PACKET_SIZE: usize = 188;

/// TS Null Packet.
/// Null packets are intended for padding of Transport Streams.
pub const NULL_PACKET: TsPacketRef = TsPacketRef(&[
    0x47, 0x1F, 0xFF, 0x10, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
]);

pub struct TsPacketRef<'a>(&'a [u8; PACKET_SIZE]);

impl<'a> TsPacketRef<'a> {
    /// Returns `true` if packet has valid sync byte.
    pub fn is_sync(&self) -> bool {
        self.0[0] == SYNC_BYTE
    }

    /// Returns `true` if transport error indicator is set (the packet was
    /// flagged as broken by the demodulator; its fields are unreliable).
    pub fn is_error(&self) -> bool {
        (self.0[1] & 0x80) != 0x00
    }

    /// Returns `true` if payload begins in the packet.
    /// TS packets with PSI and PUSI bit also contains `pointer field` in `packet[4]`.
    /// Pointer field is a offset value, if `0` then payload starts immediately after it.
    pub fn is_payload_start(&self) -> bool {
        (self.0[1] & 0x40) != 0x00
    }

    /// Returns PID - TS Packet identifier
    pub fn pid(&self) -> u16 {
        (u16::from(self.0[1] & 0x1F) << 8) | u16::from(self.0[2])
    }

    /// Returns transport scrambling control.
    pub fn scrambling_control(&self) -> u8 {
        (self.0[3] & 0xC0) >> 6
    }

    /// Returns CC - TS Packet Continuity Counter
    /// Continuity Counter is a 4-bit field incrementing with each TS packet with the same PID
    pub fn cc(&self) -> u8 {
        self.0[3] & 0x0F
    }

    /// Returns adaptation field.
    /// Returns `None` if the field length does not fit the packet (malformed).
    pub fn adaptation_field(&self) -> Option<AdaptationFieldRef<'_>> {
        let af_flag = (self.0[3] & 0x20) != 0;
        let af_size = self.0[4] as usize;
        (af_flag && PACKET_SIZE >= 5 + af_size)
            .then(|| AdaptationFieldRef(&self.0[5 .. 5 + af_size]))
    }

    /// Returns payload slice.
    pub fn payload(&self) -> Option<&'_ [u8]> {
        let af_control = (self.0[3] & 0x30) >> 4;
        if af_control & 0x1 == 0 {
            return None;
        }
        let header_skip = if af_control & 0x02 != 0 {
            4 + 1 + self.0[4] as usize
        } else {
            4
        };
        if header_skip >= PACKET_SIZE {
            return None;
        }
        Some(&self.0[header_skip ..])
    }
}

impl AsRef<[u8; PACKET_SIZE]> for TsPacketRef<'_> {
    fn as_ref(&self) -> &[u8; PACKET_SIZE] {
        self.0
    }
}

impl<'a> From<&'a [u8; PACKET_SIZE]> for TsPacketRef<'a> {
    fn from(value: &'a [u8; PACKET_SIZE]) -> Self {
        TsPacketRef(value)
    }
}

impl<'a> From<TsPacketMut<'a>> for TsPacketRef<'a> {
    fn from(value: TsPacketMut<'a>) -> Self {
        TsPacketRef(value.0)
    }
}

pub struct TsPacketMut<'a>(&'a mut [u8; PACKET_SIZE]);

impl<'a> TsPacketMut<'a> {
    /// Initializes a TS packet with sync byte and zeroed header.
    pub fn init(&mut self, pid: u16, cc: u8) {
        self.0[0] = SYNC_BYTE;
        self.0[1] = 0;
        self.0[2] = 0;
        self.0[3] = 0;

        self.set_pid(pid);
        self.set_cc(cc);
    }

    /// Sets PID value.
    pub fn set_pid(&mut self, pid: u16) {
        let pid = pid & PID_NULL;
        self.0[1] = (self.0[1] & 0xE0) | ((pid >> 8) as u8);
        self.0[2] = pid as u8;
    }

    /// Sets continuity counter value.
    pub fn set_cc(&mut self, cc: u8) {
        let cc = cc & 0x0F;
        self.0[3] = (self.0[3] & 0xF0) | cc;
    }

    /// Sets payload flag in adaptation field control.
    pub fn set_payload(&mut self) {
        self.0[3] |= 0x10
    }

    /// Clears payload flag in adaptation field control.
    pub fn clear_payload(&mut self) {
        self.0[3] &= !0x10
    }

    /// Sets payload unit start indicator (PUSI) flag in header.
    pub fn set_pusi(&mut self) {
        self.0[1] |= 0x40
    }

    /// Clears payload unit start indicator (PUSI) flag in header.
    pub fn clear_pusi(&mut self) {
        self.0[1] &= !0x40
    }

    /// Returns mutable payload slice.
    pub fn payload_mut(&mut self) -> Option<&mut [u8]> {
        let af_control = (self.0[3] & 0x30) >> 4;
        if af_control & 0x1 == 0 {
            return None;
        }
        let header_skip = if af_control & 0x02 != 0 {
            4 + 1 + self.0[4] as usize
        } else {
            4
        };
        if header_skip >= PACKET_SIZE {
            return None;
        }
        Some(&mut self.0[header_skip ..])
    }

    /// Sets adaptation field
    ///
    /// If `size == 0`, the adaptation field is removed
    /// If `size == 1`, writes an empty adaptation field (length byte `0x00`)
    /// If `size >= 2`, writes an adaptation field with length `size - 1` and flags byte `0x00`,
    /// and fills the remaining space `size - 2` with `0xFF` stuffing bytes.
    pub fn set_adaptation_field(&mut self, size: usize) {
        if size == 0 {
            self.0[3] &= !0x20;
            return;
        }

        self.0[3] |= 0x20;

        // No flags, only length byte
        if size == 1 {
            self.0[4] = 0;
            return;
        }

        self.0[4] = (size - 1) as u8;
        self.0[5] = 0x00;

        if size > 2 {
            // Limit stuffing size to maximum possible
            let end = (4 + size).min(PACKET_SIZE);
            self.0[6 .. end].copy_from_slice(&NULL_PACKET.as_ref()[6 .. end]);
        }
    }

    /// Sets PCR value in-place in the adaptation field.
    ///
    /// The adaptation field must already be present and large enough for PCR
    /// (e.g. via [`set_adaptation_field`](Self::set_adaptation_field)).
    pub fn set_pcr(&mut self, val: u64) {
        let val = val % PCR_NONE;

        let pcr_base = val / 300;
        let pcr_ext = val % 300;

        let bytes = ((pcr_base << 15) | (0x3F << 9) | pcr_ext).to_be_bytes();

        self.0[5] |= 0x10;
        self.0[6 .. 12].copy_from_slice(&bytes[2 .. 8]);
    }

    /// Sets `random_access_indicator` flag in the adaptation field.
    /// The adaptation field must already be present (e.g. via
    /// [`set_adaptation_field`](Self::set_adaptation_field)).
    pub fn set_rai(&mut self) {
        self.0[5] |= 0x40;
    }

    /// Sets `discontinuity_indicator` flag in the adaptation field.
    /// The adaptation field must already be present with a non-zero length
    /// (e.g. via [`set_adaptation_field`](Self::set_adaptation_field)).
    pub fn set_discontinuity(&mut self) {
        self.0[5] |= 0x80;
    }

    /// Removes the PCR field from the adaptation field.
    ///
    /// Clears the PCR flag and moves the optional fields that follow the PCR
    /// (OPCR, splice countdown, transport private data, adaptation field
    /// extension) 6 bytes toward the header; the freed 6 bytes become `0xFF`
    /// stuffing. The adaptation field length byte stays unchanged.
    ///
    /// Returns `false` and leaves the packet unchanged when there is no
    /// adaptation field, the field is shorter than 7 bytes, the PCR flag is
    /// clear, or the declared optional fields do not fit inside the
    /// adaptation field (malformed).
    pub fn clear_pcr(&mut self) -> bool {
        if (self.0[3] & 0x20) == 0 {
            return false;
        }

        let af_len = self.0[4] as usize;
        if !(7 ..= PACKET_SIZE - 5).contains(&af_len) {
            return false;
        }

        let flags = self.0[5];
        if (flags & 0x10) == 0 {
            return false;
        }

        // Offset within the adaptation field body (after the length byte):
        // flags byte at 0, PCR at 1 .. 7, optional fields follow
        let mut end = 7;

        // OPCR
        if (flags & 0x08) != 0 {
            end += 6;
        }

        // splice_countdown
        if (flags & 0x04) != 0 {
            end += 1;
        }

        // transport_private_data_length + private data
        if (flags & 0x02) != 0 {
            if end >= af_len {
                return false;
            }
            end += 1 + self.0[5 + end] as usize;
        }

        // adaptation_field_extension_length + extension
        if (flags & 0x01) != 0 {
            if end >= af_len {
                return false;
            }
            end += 1 + self.0[5 + end] as usize;
        }

        if end > af_len {
            return false;
        }

        self.0[5] = flags & !0x10;
        // Shift the optional fields 6 bytes toward the header and turn the
        // freed bytes into stuffing
        self.0.copy_within(12 .. 5 + end, 6);
        self.0[5 + end - 6 .. 5 + end].fill(0xFF);

        true
    }
}

impl AsRef<[u8; PACKET_SIZE]> for TsPacketMut<'_> {
    fn as_ref(&self) -> &[u8; PACKET_SIZE] {
        self.0
    }
}

impl<'a> From<&'a mut [u8; PACKET_SIZE]> for TsPacketMut<'a> {
    fn from(value: &'a mut [u8; PACKET_SIZE]) -> Self {
        TsPacketMut(value)
    }
}

impl<'a> TryFrom<&'a mut [u8]> for TsPacketMut<'a> {
    type Error = std::array::TryFromSliceError;

    fn try_from(value: &'a mut [u8]) -> Result<Self, Self::Error> {
        Ok(TsPacketMut(value.try_into()?))
    }
}

/// Builds an adaptation-field-only TS packet carrying a PCR.
///
/// The packet has no payload (`adaptation_field_control` is '10'), the
/// adaptation field fills the whole packet body (length byte 183) and
/// `transport_scrambling_control` is 00. An adaptation-field-only packet does
/// not increment the continuity counter, so `cc` should repeat the last
/// payload packet CC on the PID. When `discontinuity` is `true` the
/// `discontinuity_indicator` flag is set.
///
/// # Example
///
/// ```
/// use libmpegts::ts::{self, PACKET_SIZE, TsPacketRef};
///
/// let mut packet = [0u8; PACKET_SIZE];
/// ts::build_pcr_packet(&mut packet, 256, 5, 86405647, false);
///
/// let ts_ref = TsPacketRef::from(&packet);
/// assert_eq!(ts_ref.adaptation_field().unwrap().pcr(), Some(86405647));
/// ```
pub fn build_pcr_packet(
    packet: &mut [u8; PACKET_SIZE],
    pid: u16,
    cc: u8,
    pcr: u64,
    discontinuity: bool,
) {
    let mut ts = TsPacketMut::from(packet);
    ts.init(pid, cc);
    ts.set_adaptation_field(PACKET_SIZE - 4);
    ts.set_pcr(pcr);

    if discontinuity {
        ts.set_discontinuity();
    }
}

/// Struct to debug adaptation field
pub struct AdaptationFieldRef<'a>(&'a [u8]);

impl<'a> AdaptationFieldRef<'a> {
    /// Returns adaptation field length
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if adaptation field is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Gets discontinuity indicator.
    /// An empty adaptation field (length 0, stuffing) carries no flags.
    pub fn discontinuity_indicator(&self) -> bool {
        !self.0.is_empty() && (self.0[0] & 0x80) != 0x00
    }

    /// Gets PCR value if exists
    pub fn pcr(&self) -> Option<u64> {
        if self.0.len() < 7 || (self.0[0] & 0x10) == 0 {
            return None;
        }

        let mut bytes = [0u8; 8];
        bytes[2 .. 8].copy_from_slice(&self.0[1 .. 7]);
        let val = u64::from_be_bytes(bytes);

        let pcr_base = val >> 15;
        let pcr_ext = (val & 0x1FF).min(299);

        Some(pcr_base * 300 + pcr_ext)
    }
}

impl<'a> fmt::Debug for AdaptationFieldRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut s = f.debug_struct("AdaptationFieldRef");
        s.field("adaptation_field_length", &self.0.len());
        if self.0.is_empty() {
            return s.finish();
        }

        s.field("discontinuity_indicator", &((self.0[0] & 0x80) >> 7));
        s.field("random_access_indicator", &((self.0[0] & 0x40) >> 6));
        s.field(
            "elementary_stream_priority_indicator",
            &((self.0[0] & 0x20) >> 5),
        );
        s.field("pcr_flag", &((self.0[0] & 0x10) >> 4));
        s.field("opcr_flag", &((self.0[0] & 0x08) >> 3));
        s.field("splicing_point_flag", &((self.0[0] & 0x04) >> 2));
        s.field("transport_private_data_flag", &((self.0[0] & 0x02) >> 1));
        s.field("adaptation_field_extension_flag", &(self.0[0] & 0x01));

        s.field("pcr", &self.pcr());

        s.finish()
    }
}

impl<'a> fmt::Debug for TsPacketRef<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TsPacketRef")
            .field("sync_byte", &self.0[0])
            .field("transport_error_indicator", &((self.0[1] & 0x80) >> 7))
            .field("payload_unit_start_indicator", &((self.0[1] & 0x40) >> 6))
            .field("transport_priority", &((self.0[1] & 0x20) >> 5))
            .field("pid", &self.pid())
            .field("transport_scrambling_control", &self.scrambling_control())
            .field("adaptation_field_control", &((self.0[3] & 0x30) >> 4))
            .field("continuity_counter", &self.cc())
            .field("adaptation_field", &self.adaptation_field())
            .finish()
    }
}
