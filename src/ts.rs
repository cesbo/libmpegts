use std::fmt;


pub const PID_NONE: u16 = 8192;
pub const PID_NULL: u16 = (PID_NONE - 1);
pub const PACKET_SIZE: usize = 188;


/// PCR - Program Clock Reference
/// 27clocks = 1us
pub const PCR_US_CLOCK: u64 = 27;
pub const PCR_SYSTEM_CLOCK: u64 = PCR_US_CLOCK * 1_000_000;
pub const PCR_NONE: u64 = (1 << 33) * 300;
pub const PCR_MAX: u64 = PCR_NONE - 1;


/// TS Null Packet.
/// Null packets are intended for padding of Transport Streams.
pub static NULL_PACKET: &[u8] = &[
    0x47, 0x1F, 0xFF, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
];


/// Hack for TS packet padding
pub static FILL_PACKET: &[u8] = &[
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
    0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF
];


pub struct TsAdaptation<'a>(&'a [u8]);


impl<'a> TsAdaptation<'a> {
    #[inline]
    pub fn new(packet: &'a [u8]) -> Self {
        debug_assert!(packet.len() >= PACKET_SIZE);
        TsAdaptation(packet)
    }
}


impl<'a> fmt::Debug for TsAdaptation<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if ! is_adaptation(self.0) {
            return fmt::Debug::fmt(&false, f)
        }

        let mut s = f.debug_struct("TsAdaptation");
        let len = get_adaptation_size(self.0);
        s.field("length", &len);
        if len == 0 {
            return s.finish()
        }

        let p = &(self.0)[5 ..];
        let pcr_flag = (p[0] & 0x10) != 0;

        s.field("discontinuity", &((p[0] & 0x80) != 0));
        s.field("random_access", &((p[0] & 0x40) != 0));
        s.field("es_priority", &((p[0] & 0x20) != 0));
        s.field("PCR_flag", &pcr_flag);
        s.field("OPCR_flag", &((p[0] & 0x08) != 0));
        s.field("splicing_point", &((p[0] & 0x04) != 0));
        s.field("private_data", &((p[0] & 0x02) != 0));
        s.field("af_extension", &((p[0] & 0x01) != 0));

        if pcr_flag {
            let p = &(self.0)[6 ..];
            s.field("pcr_base", &get_pcr_base(p));
            s.field("pcr_ext", &get_pcr_ext(p));
        }

        s.finish()
    }
}


pub struct TsPacket<'a>(&'a [u8]);


impl<'a> TsPacket<'a> {
    #[inline]
    pub fn new(packet: &'a [u8]) -> Self {
        debug_assert!(packet.len() >= PACKET_SIZE);
        TsPacket(packet)
    }
}


impl<'a> fmt::Debug for TsPacket<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("TsPacket")
            .field("sync", &is_sync(self.0))
            .field("error", &(((self.0)[1] & 0x80) >> 7))
            .field("pusi", &is_pusi(self.0))
            .field("pid", &get_pid(self.0))
            .field("scrambling", &(((self.0)[3] & 0xC0) >> 6))
            .field("adaptation", &TsAdaptation::new(self.0))
            .field("payload", &is_payload(self.0))
            .field("cc", &get_cc(self.0))
            .finish()
    }
}


/// Returns `true` if packet has valid sync byte.
#[inline]
pub fn is_sync(ts: &[u8]) -> bool { ts[0] == 0x47 }


/// Returns `true` if the transport error indicator is set
#[inline]
pub fn is_error(ts: &[u8]) -> bool { (ts[1] & 0x80) != 0x00 }


/// Returns `true` if packet contains payload.
#[inline]
pub fn is_payload(ts: &[u8]) -> bool { (ts[3] & 0x10) != 0x00 }


/// Returns `true` if payload begins in the packet.
/// TS packets with PSI and PUSI bit also contains `pointer field` in `packet[4]`.
/// Pointer field is a offset value, if `0` then payload starts immediately after it.
#[inline]
pub fn is_pusi(ts: &[u8]) -> bool { (ts[1] & 0x40) != 0x00 }


/// Returns `true` if packet contain adaptation field.
/// Adaptation field locates after TS header.
#[inline]
pub fn is_adaptation(ts: &[u8]) -> bool { (ts[3] & 0x20) != 0x00 }


/// Returns payload offset in the TS packet
/// Sum of the TS header size and adaptation field if exists.
/// If TS packet without payload or offset value is invalid returns `0`
/// In the PSI packets the `pointer field` is a part of payload, so it do not sums.
///
/// # Examples
///
/// ```
/// use mpegts::ts::*;
/// let packet: Vec<u8> = vec![0x47, 0x40, 0x11, 0x10, 0x00, /* ... */];
/// assert!(is_payload(&packet));
/// assert_eq!(get_payload_offset(&packet), 4);
/// let packet: Vec<u8> = vec![0x47, 0x40, 0x2d, 0xf0, 0x19, 0x00, /* ... */];
/// assert!(is_payload(&packet));
/// assert_eq!(get_payload_offset(&packet), 4 + 1 + 0x19);
/// ```
#[inline]
pub fn get_payload_offset(ts: &[u8]) -> u8 {
    if is_adaptation(ts) {
        4 + 1 + get_adaptation_size(ts)
    } else {
        4
    }
}


/// Returns `true` if the payload is scrambled.
/// Actually this is only flag and packet contain could be not scrambled.
#[inline]
pub fn is_scrambled(ts: &[u8]) -> bool { (ts[3] & 0xC0) != 0 }


/// Returns the size of the adaptation field.
/// Function should be used if [`is_adaptation`] is `true`
///
/// [`is_adaptation`]: #method.is_adaptation
#[inline]
pub fn get_adaptation_size(ts: &[u8]) -> u8 { ts[4] }


/// Returns PID - TS Packet identifier
#[inline]
pub fn get_pid(ts: &[u8]) -> u16 { (u16::from(ts[1] & 0x1F) << 8) | u16::from(ts[2]) }


/// Returns CC - TS Packet Continuity Counter
/// Continuity Counter is a 4-bit field incrementing with each TS packet with the same PID
#[inline]
pub fn get_cc(ts: &[u8]) -> u8 { ts[3] & 0x0F }


/// Allocates Vec<u8>
pub fn new_ts() -> Vec<u8> {
    let mut ts: Vec<u8> = Vec::new();
    ts.resize(PACKET_SIZE, 0x00);
    ts[0] = 0x47;
    ts
}


/// Sets PID
///
/// # Examples
///
/// ```
/// use mpegts::ts::*;
/// let mut ts = new_ts();
/// set_pid(&mut ts, 8191);
/// assert_eq!(get_pid(&ts), 8191);
/// ```
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
    ts[3] &= !0x10;
}


#[inline]
pub fn set_payload_1(ts: &mut [u8]) {
    ts[3] |= 0x10;
}


#[inline]
pub fn set_pusi_0(ts: &mut [u8]) {
    ts[1] &= !0x40;
}


#[inline]
pub fn set_pusi_1(ts: &mut [u8]) {
    ts[1] |= 0x40;
}


/// Returns `true` if TS packet has PCR field
///
/// # Examples
///
/// ```
/// use mpegts::ts::*;
///
/// let packet: Vec<u8> = vec![0x47, 0x01, 0x00, 0x20, 0xb7, 0x10, /* ... */];
/// assert!(is_pcr(&packet));
///
/// let packet: Vec<u8> = vec![0x47, 0x40, 0x11, 0x10, 0x00, /* ... */];
/// assert!(!is_pcr(&packet));
/// ```
#[inline]
pub fn is_pcr(ts: &[u8]) -> bool {
    is_adaptation(ts) && get_adaptation_size(ts) >= 7 && (ts[5] & 0x10) != 0
}


#[inline]
fn get_pcr_base(ts: &[u8]) -> u64 {
    (u64::from(ts[0]) << 25) |
    (u64::from(ts[1]) << 17) |
    (u64::from(ts[2]) <<  9) |
    (u64::from(ts[3]) <<  1) |
    (u64::from(ts[4]) >>  7)
}


#[inline]
fn get_pcr_ext(ts: &[u8]) -> u64 {
    (u64::from(ts[4] & 0x01) << 8) | u64::from(ts[5])
}


/// Gets PCR value
///
/// # Examples
///
/// ```
/// use mpegts::ts::*;
/// let packet: Vec<u8> = vec![
///     0x47, 0x01, 0x00, 0x20, 0xb7, 0x10, 0x00, 0x02, 0x32, 0x89, 0x7e, 0xf7, /* ... */];
/// assert!(is_pcr(&packet));
/// assert_eq!(get_pcr(&packet), 86405647);
/// ```
#[inline]
pub fn get_pcr(ts: &[u8]) -> u64 {
    let p = &ts[6 .. 12];
    get_pcr_base(p) * 300 + get_pcr_ext(p)
}


/// Returns difference between previous PCR and current PCR
///
/// # Examples
///
/// ```
/// use mpegts::ts::*;
///
/// let current_pcr = 20000;
/// let last_pcr = current_pcr - 10000;
/// assert_eq!(pcr_delta(last_pcr, current_pcr), 10000);
///
/// let current_pcr = 5000;
/// let last_pcr = PCR_MAX - 5000;
/// assert_eq!(pcr_delta(last_pcr, current_pcr), 10000);
/// ```
#[inline]
pub fn pcr_delta(last_pcr: u64, current_pcr: u64) -> u64 {
    if current_pcr >= last_pcr {
        current_pcr - last_pcr
    } else {
        current_pcr + PCR_MAX - last_pcr
    }
}


/// Convert PCR to milliseconds
#[inline]
pub fn pcr_to_ms(pcr: u64) -> u64 {
    (pcr) / (PCR_SYSTEM_CLOCK / 1_000)
}


/// Get PCR bitrate
#[inline]
pub fn pcr_delta_bitrate(delta: u64, bytes: u64) -> u64 {
    (bytes * 8) / pcr_to_ms(delta)
}
