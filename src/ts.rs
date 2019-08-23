// Copyright (C) 2018-2019 Cesbo OU <info@cesbo.com>
//
// This file is part of ASC/libmpegts
//
// ASC/libmpegts can not be copied and/or distributed without the express
// permission of Cesbo OU

use std::fmt;


pub const PID_NONE: u16 = 8192;
pub const PID_NULL: u16 = (PID_NONE - 1);
pub const PACKET_SIZE: usize = 188;


/// PCR - Program Clock Reference
/// 27clocks = 1us
pub const PCR_CLOCK_US: u64 = 27;
pub const PCR_CLOCK_MS: u64 = PCR_CLOCK_US * 1_000;
pub const PCR_SYSTEM_CLOCK: u64 = PCR_CLOCK_US * 1_000_000;
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


/// Struct to debug adaptation field
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
            s.field("pcr", &get_pcr(self.0));
        }

        s.finish()
    }
}


/// Struct to debug TS packet header
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
#[inline]
pub fn get_payload_offset(ts: &[u8]) -> u8 {
    if ! is_adaptation(ts) {
        4
    } else {
        4 + 1 + get_adaptation_size(ts)
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
#[inline]
pub fn is_pcr(ts: &[u8]) -> bool {
    is_adaptation(ts) && get_adaptation_size(ts) >= 7 && (ts[5] & 0x10) != 0
}


/// Sets PCR value
#[inline]
pub fn set_pcr(ts: &mut [u8], pcr: u64) {
    let pcr_base = pcr / 300;
    let pcr_ext = pcr % 300;

    ts[6] = ((pcr_base >> 25) & 0xFF) as u8;
    ts[7] = ((pcr_base >> 17) & 0xFF) as u8;
    ts[8] = ((pcr_base >> 9) & 0xFF) as u8;
    ts[9] = ((pcr_base >> 1) & 0xFF) as u8;
    ts[10] = (((pcr_base << 7) & 0x80) as u8) | 0x7E | (((pcr_ext >> 8) & 0x01) as u8);
    ts[11] = (pcr_ext & 0xFF) as u8;
}


/// Gets PCR value
#[inline]
pub fn get_pcr(ts: &[u8]) -> u64 {
    let pcr_base =
        (u64::from(ts[6]) << 25) |
        (u64::from(ts[7]) << 17) |
        (u64::from(ts[8]) <<  9) |
        (u64::from(ts[9]) <<  1) |
        (u64::from(ts[10]) >>  7);

    let pcr_ext =
        (u64::from(ts[10] & 0x01) << 8) | u64::from(ts[11]);

    pcr_base * 300 + pcr_ext
}


/// Returns difference between previous PCR and current PCR
#[inline]
pub fn pcr_delta(last_pcr: u64, current_pcr: u64) -> u64 {
    if current_pcr >= last_pcr {
        current_pcr - last_pcr
    } else {
        current_pcr + PCR_MAX - last_pcr
    }
}


/// Calculate STC (System Time Clock) value
///
/// STC is an estimated value for current PCR
///
/// ```ignore
/// |time:-->                     |
/// |----A---------B---------C----|
///       \         \         \
///        \         \         pcr_c - current PCR
///         \         pcr_b
///          pcr_a
///
/// last_bytes  - bytes between pcr_b and pcr_a
/// bytes       - bytes between pcr_c and pcr_b
///
/// (STC - pcr_b)      bytes
/// --------------- == ----------
/// (pcr_b - pcr_a)    last_bytes
/// ```
///
/// ## Example
///
/// ```
/// use mpegts::ts;
///
/// let pcr_a = 354923263808u64;
/// let pcr_b = 354924281094u64;
/// let last_bytes = 7708;
/// let bytes = 7520;
///
/// let stc = ts::pcr_to_stc(pcr_b, bytes, pcr_b - pcr_a, last_bytes);
/// assert_eq!(stc, 354925273568u64);
/// ```
#[inline]
pub fn pcr_to_stc(last_pcr: u64, bytes: u64, last_delta: u64, last_bytes: u64) -> u64 {
    last_delta * bytes / last_bytes + last_pcr
}


/// Calculate PCR jitter in ns
#[inline]
pub fn pcr_jitter_ns(pcr: u64, stc: u64) -> i64 {
    let mut result = {
        if pcr < stc {
            pcr + PCR_MAX - stc
        } else {
            pcr - stc
        }
    } as i64;

    if result > PCR_SYSTEM_CLOCK as i64 {
        result -= PCR_MAX as i64;
    }

    result * 1000 / PCR_CLOCK_US as i64
}


/// Converts PCR to microseconds
#[inline]
pub fn pcr_to_us(pcr: u64) -> u64 { pcr / PCR_CLOCK_US }


/// Converts PCR to milliseconds
#[inline]
pub fn pcr_to_ms(pcr: u64) -> u64 { pcr / PCR_CLOCK_MS }


/// Claclulate PCR bitrate
#[inline]
pub fn pcr_delta_bitrate(delta: u64, bytes: u64) -> u64 {
    (bytes * 8) / pcr_to_ms(delta)
}


#[cfg(test)]
mod tests {
    use crate::ts;

    #[test]
    fn test_get_payload_offset() {
        let packet: Vec<u8> = vec![0x47, 0x40, 0x11, 0x10, 0x00];
        assert!(ts::is_payload(&packet));
        assert_eq!(ts::get_payload_offset(&packet), 4);

        let packet: Vec<u8> = vec![0x47, 0x40, 0x2d, 0xf0, 0x19, 0x00];
        assert!(ts::is_payload(&packet));
        assert_eq!(ts::get_payload_offset(&packet), 4 + 1 + 0x19);
    }

    #[test]
    fn test_set_payload_1() {
        let mut packet = vec![0x47, 0x00, 0x00, 0x00];
        ts::set_payload_1(&mut packet);
        assert_eq!(packet[3], 0x10);
    }

    #[test]
    fn test_set_payload_0() {
        let mut packet = vec![0x47, 0x00, 0x00, 0xFF];
        ts::set_payload_0(&mut packet);
        assert_eq!(packet[3], 0xEF);
    }

    #[test]
    fn test_set_pusi_1() {
        let mut packet = vec![0x47, 0x00];
        ts::set_pusi_1(&mut packet);
        assert_eq!(packet[1], 0x40);
    }

    #[test]
    fn test_set_pusi_0() {
        let mut packet = vec![0x47, 0xFF];
        ts::set_pusi_0(&mut packet);
        assert_eq!(packet[1], 0xBF);
    }

    #[test]
    fn test_set_pid() {
        let mut packet = vec![0x47, 0x00, 0x00];
        ts::set_pid(&mut packet, 8191);
        assert_eq!(&[0x1F, 0xFF], &packet[1..]);
    }

    #[test]
    fn test_is_pcr() {
        let packet: Vec<u8> = vec![0x47, 0x01, 0x00, 0x20, 0xb7, 0x10];
        assert!(ts::is_pcr(&packet));

        let packet: Vec<u8> = vec![0x47, 0x40, 0x11, 0x10, 0x00];
        assert!(!ts::is_pcr(&packet));
    }

    #[test]
    fn test_pcr_delta() {
        let current_pcr = 20000;
        let last_pcr = current_pcr - 10000;
        assert_eq!(ts::pcr_delta(last_pcr, current_pcr), 10000);
    }

    #[test]
    fn test_pcr_delta_overflow() {
        let current_pcr = 5000;
        let last_pcr = ts::PCR_MAX - 5000;
        assert_eq!(ts::pcr_delta(last_pcr, current_pcr), 10000);
    }

    #[test]
    fn test_get_pcr() {
        let packet: Vec<u8> = vec![
            0x47, 0x01, 0x00, 0x20, 0xB7, 0x10, 0x00, 0x02, 0x32, 0x89, 0x7E, 0xF7];
        assert!(ts::is_pcr(&packet));
        assert_eq!(ts::get_pcr(&packet), 86405647);
    }

    #[test]
    fn test_set_pcr() {
        let mut packet: Vec<u8> = vec![
            0x47, 0x01, 0x00, 0x20, 0xB7, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        ts::set_pcr(&mut packet, 86405647);
        assert_eq!(&[0x00, 0x02, 0x32, 0x89, 0x7E, 0xF7], &packet[6 ..]);
    }
}
