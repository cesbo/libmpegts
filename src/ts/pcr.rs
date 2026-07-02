//! PCR - Program Clock Reference

pub const PCR_CLOCK_US: u64 = 27; // 27clocks = 1us
pub const PCR_CLOCK_MS: u64 = PCR_CLOCK_US * 1_000;
pub const PCR_SYSTEM_CLOCK: u64 = PCR_CLOCK_US * 1_000_000;
pub const PCR_NONE: u64 = (1 << 33) * 300;
pub const PCR_MAX: u64 = PCR_NONE - 1;

/// Returns difference between previous PCR and current PCR.
pub fn pcr_delta(last_pcr: u64, current_pcr: u64) -> u64 {
    if current_pcr >= last_pcr {
        current_pcr - last_pcr
    } else {
        current_pcr + PCR_NONE - last_pcr
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
/// use libmpegts::ts;
///
/// let pcr_a = 354923263808u64;
/// let pcr_b = 354924281094u64;
/// let last_bytes = 7708;
/// let bytes = 7520;
///
/// let stc = ts::pcr_to_stc(pcr_b, bytes, pcr_b - pcr_a, last_bytes);
/// assert_eq!(stc, 354925273568u64);
/// ```
pub fn pcr_to_stc(last_pcr: u64, bytes: u64, last_delta: u64, last_bytes: u64) -> u64 {
    last_delta * bytes / last_bytes + last_pcr
}

/// Calculate PCR jitter in ns
pub fn pcr_jitter_ns(pcr: u64, stc: u64) -> i64 {
    let mut result = {
        if pcr < stc {
            pcr + PCR_NONE - stc
        } else {
            pcr - stc
        }
    } as i64;

    if result > PCR_SYSTEM_CLOCK as i64 {
        result -= PCR_NONE as i64;
    }

    result * 1000 / PCR_CLOCK_US as i64
}

/// Converts PCR to microseconds
pub fn pcr_to_us(pcr: u64) -> u64 {
    pcr / PCR_CLOCK_US
}

/// Converts PCR to milliseconds
pub fn pcr_to_ms(pcr: u64) -> u64 {
    pcr / PCR_CLOCK_MS
}

/// Claclulate PCR bitrate
pub fn pcr_delta_bitrate(delta: u64, bytes: u64) -> u64 {
    (bytes * 8) / pcr_to_ms(delta)
}
