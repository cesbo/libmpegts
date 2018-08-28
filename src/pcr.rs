use ts::*;

pub const PCR_SYSTEM_CLOCK: u64 = 27_000_000;
pub const PCR_MAX: u64 = 0x200000000 * 300;
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
        ((ts[ 6] as u64) << 25) |
        ((ts[ 7] as u64) << 17) |
        ((ts[ 8] as u64) <<  9) |
        ((ts[ 9] as u64) <<  1) |
        ((ts[10] as u64) >>  7);
    let pcr_ext = (((ts[10] as u64) << 8) | (ts[11] as u64)) & 0x01FF;
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
