//! PCR - Program Clock Reference: clock arithmetic and synthesis.
//!
//! Clock constants and wrap-aware helpers for the 27 MHz PCR timeline, plus
//! [`PcrSynth`] - a pure per-packet pass-through stage that guarantees a
//! valid PCR in an SPTS stream whose own PCR is absent, broken, undeclared
//! (PMT `PCR_PID` set to `0x1FFF`) or too sparse. The clock is derived from
//! PES PTS/DTS in decode order and extrapolated over the byte position using
//! a decaying byte-rate window.
//!
//! Synthesis guarantees and non-goals:
//!
//! - The guarantee is PCR repetition (35 ms target between emitted PCRs in
//!   stream time), wrap-aware monotonicity within a discontinuity era and
//!   correct `discontinuity_indicator` signaling. PCR accuracy (+-500 ns,
//!   TR 101 290 PCR_AC) is NOT met by rate extrapolation; streams destined
//!   for RF or modulators should be shaped by a downstream CBR stage.
//! - MPTS is out of scope: more than one program in the PAT disables the
//!   stage ([`PcrSynthPhase::MultiProgram`]).
//! - Scrambled elementary streams cannot drive the clock (PES headers are
//!   ciphertext); writing PCR remains possible, so top-up injection still
//!   works on scrambled streams with real PCRs.

mod synth;

pub use synth::*;

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
/// use libmpegts::pcr;
///
/// let pcr_a = 354923263808u64;
/// let pcr_b = 354924281094u64;
/// let last_bytes = 7708;
/// let bytes = 7520;
///
/// let stc = pcr::pcr_to_stc(pcr_b, bytes, pcr_b - pcr_a, last_bytes);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pcr_delta() {
        let current_pcr = 20000;
        let last_pcr = current_pcr - 10000;
        assert_eq!(pcr_delta(last_pcr, current_pcr), 10000);

        assert_eq!(pcr_delta(20000, 20000), 0);
    }

    #[test]
    fn test_pcr_delta_overflow() {
        // the PCR ring is modulo PCR_NONE: from PCR_MAX - 5000 the counter takes
        // 5001 ticks to wrap to 0 and 5000 more to reach 5000
        let current_pcr = 5000;
        let last_pcr = PCR_MAX - 5000;
        assert_eq!(pcr_delta(last_pcr, current_pcr), 10001);

        // one tick from the last value to the wrapped zero
        assert_eq!(pcr_delta(PCR_MAX, 0), 1);

        // almost a full circle
        assert_eq!(pcr_delta(1, 0), PCR_MAX);
    }
}
