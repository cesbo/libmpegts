pub const PID_NONE: u16 = 8192;
pub const PID_NULL: u16 = PID_NONE - 1;
pub const PACKET_SIZE: usize = 188;

pub trait TsPacketsExt {
    fn ts_packets(&self) -> impl Iterator<Item = TsPacketRef<'_>>;
}

impl TsPacketsExt for [u8] {
    fn ts_packets(&self) -> impl Iterator<Item = TsPacketRef<'_>> {
        let n = if self.len() % PACKET_SIZE == 0 {
            self.len() / PACKET_SIZE
        } else {
            0
        };
        let ptr = self.as_ptr() as *const [u8; PACKET_SIZE];
        let batch = unsafe { std::slice::from_raw_parts(ptr, n) };
        batch.iter().map(TsPacketRef)
    }
}

pub struct TsPacketRef<'a>(&'a [u8; PACKET_SIZE]);

impl<'a> TsPacketRef<'a> {
    /// Returns `true` if packet has valid sync byte.
    #[inline]
    pub fn is_sync(&self) -> bool {
        self.0[0] == 0x47
    }

    /// Returns `true` if the transport error indicator is set
    #[inline]
    pub fn is_error(&self) -> bool {
        (self.0[1] & 0x80) != 0x00
    }

    /// Returns `true` if packet contains payload.
    #[inline]
    pub fn is_payload(&self) -> bool {
        (self.0[3] & 0x10) != 0x00
    }

    /// Returns `true` if payload begins in the packet.
    /// TS packets with PSI and PUSI bit also contains `pointer field` in `packet[4]`.
    /// Pointer field is a offset value, if `0` then payload starts immediately after it.
    #[inline]
    pub fn is_pusi(&self) -> bool {
        (self.0[1] & 0x40) != 0x00
    }

    /// Returns `true` if packet contain adaptation field.
    /// Adaptation field locates after TS header.
    #[inline]
    pub fn is_adaptation(&self) -> bool {
        (self.0[3] & 0x20) != 0x00
    }

    /// Returns payload offset in the TS packet
    /// Sum of the TS header size and adaptation field if exists.
    /// If TS packet without payload or offset value is invalid returns `0`
    /// In the PSI packets the `pointer field` is a part of payload, so it do not sums.
    #[inline]
    pub fn payload_offset(&self) -> u8 {
        if !self.is_adaptation() {
            4
        } else {
            4 + 1 + self.adaptation_size()
        }
    }

    /// Returns `true` if the payload is scrambled.
    /// Actually this is only flag and packet contain could be not scrambled.
    #[inline]
    pub fn is_scrambled(&self) -> bool {
        (self.0[3] & 0xC0) != 0
    }

    /// Returns the size of the adaptation field.
    /// Function should be used if [`is_adaptation`] is `true`
    ///
    /// [`is_adaptation`]: #method.is_adaptation
    #[inline]
    pub fn adaptation_size(&self) -> u8 {
        self.0[4]
    }

    /// Returns PID - TS Packet identifier
    #[inline]
    pub fn pid(&self) -> u16 {
        (u16::from(self.0[1] & 0x1F) << 8) | u16::from(self.0[2])
    }

    /// Returns CC - TS Packet Continuity Counter
    /// Continuity Counter is a 4-bit field incrementing with each TS packet with the same PID
    #[inline]
    pub fn cc(&self) -> u8 {
        self.0[3] & 0x0F
    }

    /// Returns `true` if TS packet has PCR field
    #[inline]
    pub fn is_pcr(&self) -> bool {
        self.is_adaptation() && self.adaptation_size() >= 7 && (self.0[5] & 0x10) != 0
    }

    /// Gets PCR value
    #[inline]
    pub fn get_pcr(&self) -> u64 {
        let mut bytes = [0u8; 8];
        bytes[2 .. 8].copy_from_slice(&self.0[6 .. 12]);
        let val = u64::from_be_bytes(bytes);

        let pcr_base = val >> 15;
        let pcr_ext = (val & 0x1FF).min(299);

        pcr_base * 300 + pcr_ext
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

/// Returns `true` if packet has valid sync byte.
#[inline]
pub fn is_sync(ts: &[u8]) -> bool {
    ts[0] == 0x47
}

/// Returns `true` if the transport error indicator is set
#[inline]
pub fn is_error(ts: &[u8]) -> bool {
    (ts[1] & 0x80) != 0x00
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

/// Returns `true` if the payload is scrambled.
/// Actually this is only flag and packet contain could be not scrambled.
#[inline]
pub fn is_scrambled(ts: &[u8]) -> bool {
    (ts[3] & 0xC0) != 0
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

mod debug;
pub use debug::*;

mod pcr;
pub use pcr::*;
