mod pcr;

use std::fmt;

pub use pcr::*;

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

/// Hack for TS packet padding
pub(crate) const FILL_PACKET: &[u8] = &[
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
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
];

pub struct TsPacketRef<'a>(&'a [u8; PACKET_SIZE]);

impl<'a> TsPacketRef<'a> {
    /// Returns `true` if packet has valid sync byte.
    #[inline]
    pub fn is_sync(&self) -> bool {
        self.0[0] == SYNC_BYTE
    }

    /// Returns `true` if payload begins in the packet.
    /// TS packets with PSI and PUSI bit also contains `pointer field` in `packet[4]`.
    /// Pointer field is a offset value, if `0` then payload starts immediately after it.
    #[inline]
    pub fn is_payload_start(&self) -> bool {
        (self.0[1] & 0x40) != 0x00
    }

    /// Returns PID - TS Packet identifier
    #[inline]
    pub fn pid(&self) -> u16 {
        (u16::from(self.0[1] & 0x1F) << 8) | u16::from(self.0[2])
    }

    /// Returns transport scrambling control.
    #[inline]
    pub fn scrambling_control(&self) -> u8 {
        (self.0[3] & 0xC0) >> 6
    }

    /// Returns CC - TS Packet Continuity Counter
    /// Continuity Counter is a 4-bit field incrementing with each TS packet with the same PID
    #[inline]
    pub fn cc(&self) -> u8 {
        self.0[3] & 0x0F
    }

    /// Returns adaptation field.
    #[inline]
    pub fn adaptation_field(&self) -> Option<AdaptationFieldRef<'_>> {
        let af_flag = (self.0[3] & 0x20) != 0;
        let af_size = self.0[4] as usize;
        (af_flag && af_size > 0).then(|| AdaptationFieldRef(self.0[5 .. 5 + af_size].as_ref()))
    }

    /// Returns payload slice.
    #[inline]
    pub fn payload(&self) -> Option<&'_ [u8]> {
        let af_control = (self.0[3] & 0x30) >> 4;
        if af_control & 0x1 == 0 {
            return None;
        }
        let header_skip = if af_control & 0x2 != 0 {
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
    pub fn set_pid(&mut self, pid: u16) {
        debug_assert!(pid < 8192);
        self.0[1] = (self.0[1] & 0xE0) | ((pid >> 8) as u8);
        self.0[2] = pid as u8;
    }

    pub fn set_cc(&mut self, cc: u8) {
        debug_assert!(cc < 16);
        self.0[3] = (self.0[3] & 0xF0) | (cc & 0x0F);
    }

    #[inline]
    pub fn set_payload(&mut self) {
        self.0[3] |= 0x10
    }

    #[inline]
    pub fn clear_payload(&mut self) {
        self.0[3] &= !0x10
    }

    #[inline]
    pub fn set_pusi(&mut self) {
        self.0[1] |= 0x40
    }

    #[inline]
    pub fn clear_pusi(&mut self) {
        self.0[1] &= !0x40
    }

    /// Sets PCR value
    #[inline]
    pub fn set_pcr(&mut self, val: u64) {
        let val = val % pcr::PCR_NONE;

        let pcr_base = val / 300;
        let pcr_ext = val % 300;

        let bytes = ((pcr_base << 15) | (0x3F << 9) | pcr_ext).to_be_bytes();

        self.0[6 .. 12].copy_from_slice(&bytes[2 .. 8]);
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

/// Struct to debug adaptation field
pub struct AdaptationFieldRef<'a>(&'a [u8]);

impl<'a> AdaptationFieldRef<'a> {
    /// Returns adaptation field length
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Returns `true` if adaptation field is empty
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Gets discontinuity indicator
    #[inline]
    pub fn discontinuity_indicator(&self) -> bool {
        (self.0[0] & 0x80) != 0x00
    }

    /// Gets PCR value if exists
    #[inline]
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

/// Returns `true` if packet contains payload.
#[inline]
pub fn is_payload(ts: &[u8]) -> bool {
    (ts[3] & 0x10) != 0x00
}

/// Returns `true` if payload begins in the packet.
/// TS packets with PSI and PUSI bit also contains `pointer field` in `packet[4]`.
/// Pointer field is a offset value, if `0` then payload starts immediately after it.
#[inline]
pub fn is_pusi(ts: &[u8]) -> bool {
    (ts[1] & 0x40) != 0x00
}

/// Returns `true` if packet contain adaptation field.
/// Adaptation field locates after TS header.
#[inline]
pub fn is_adaptation(ts: &[u8]) -> bool {
    (ts[3] & 0x20) != 0x00
}

/// Returns payload offset in the TS packet
/// Sum of the TS header size and adaptation field if exists.
/// If TS packet without payload or offset value is invalid returns `0`
/// In the PSI packets the `pointer field` is a part of payload, so it do not sums.
#[inline]
pub fn get_payload_offset(ts: &[u8]) -> u8 {
    if !is_adaptation(ts) {
        4
    } else {
        4 + 1 + get_adaptation_size(ts)
    }
}

/// Returns the size of the adaptation field.
/// Function should be used if [`is_adaptation`] is `true`
///
/// [`is_adaptation`]: #method.is_adaptation
#[inline]
pub fn get_adaptation_size(ts: &[u8]) -> u8 {
    ts[4]
}

/// Returns PID - TS Packet identifier
#[inline]
pub fn get_pid(ts: &[u8]) -> u16 {
    (u16::from(ts[1] & 0x1F) << 8) | u16::from(ts[2])
}

/// Returns CC - TS Packet Continuity Counter
/// Continuity Counter is a 4-bit field incrementing with each TS packet with the same PID
#[inline]
pub fn get_cc(ts: &[u8]) -> u8 {
    ts[3] & 0x0F
}

/// Sets PID
#[inline]
pub fn set_pid(ts: &mut [u8], pid: u16) {
    debug_assert!(pid < 8192);
    ts[1] = (ts[1] & 0xE0) | ((pid >> 8) as u8);
    ts[2] = pid as u8;
}

#[inline]
pub fn set_cc(ts: &mut [u8], cc: u8) {
    debug_assert!(cc < 16);
    ts[3] = (ts[3] & 0xF0) | (cc & 0x0F);
}

#[inline]
pub fn set_payload_0(ts: &mut [u8]) {
    ts[3] &= !0x10
}

#[inline]
pub fn set_payload_1(ts: &mut [u8]) {
    ts[3] |= 0x10
}

#[inline]
pub fn set_pusi_0(ts: &mut [u8]) {
    ts[1] &= !0x40
}

#[inline]
pub fn set_pusi_1(ts: &mut [u8]) {
    ts[1] |= 0x40
}
