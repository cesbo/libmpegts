use crate::ts::*;

pub const PCR_SYSTEM_CLOCK: u64 = 27_000_000;
pub const PCR_MAX: u64 = 0x0002_0000_0000 * 300;
pub const PCR_NONE: u64 = PCR_MAX + 1;

/// Returns `true` if TS packet has PCR field
///
/// # Examples
///
/// ```
/// use mpegts::pcr::*;
///
/// let packet: Vec<u8> = vec![0x47, 0x01, 0x00, 0x20, 0xb7, 0x10, /* ... */];
/// assert!(is_pcr(&packet));
///
/// let packet: Vec<u8> = vec![0x47, 0x40, 0x11, 0x10, 0x00, /* ... */];
/// assert!(!is_pcr(&packet));
/// ```
#[inline]
pub fn is_pcr(ts: &[u8]) -> bool {
    is_adaptation(ts) && get_adaptation_size(ts) > 7 && (ts[5] & 0x10 != 0)
}

/// Gets PCR value
///
/// # Examples
///
/// ```
/// use mpegts::pcr::*;
/// let packet: Vec<u8> = vec![0x47, 0x01, 0x00, 0x20, 0xb7, 0x10, 0x00, 0x02, 0x32, 0x89, 0x7e, 0xf7, /* ... */];
/// assert!(is_pcr(&packet));
/// assert_eq!(get_pcr(&packet), 86405647);
/// ```
#[inline]
pub fn get_pcr(ts: &[u8]) -> u64 {
    let pcr_base =
        (u64::from(ts[ 6]) << 25) |
        (u64::from(ts[ 7]) << 17) |
        (u64::from(ts[ 8]) <<  9) |
        (u64::from(ts[ 9]) <<  1) |
        (u64::from(ts[10]) >>  7);
    let pcr_ext = ((u64::from(ts[10]) << 8) | u64::from(ts[11])) & 0x01FF;
    pcr_base * 300 + pcr_ext
}

/// Returns difference between previous PCR and current PCR
///
/// # Examples
///
/// ```
/// use mpegts::pcr::*;
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
