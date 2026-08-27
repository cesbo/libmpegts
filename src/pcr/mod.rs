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

/// Signed difference `to - from` on the PCR ring, shortest direction; a tie
/// (exactly half the ring apart) counts as forward.
pub fn pcr_signed_delta(from: u64, to: u64) -> i64 {
    const HALF: i64 = PCR_NONE as i64 / 2;
    let d = to as i64 - from as i64;
    if d > HALF {
        d - PCR_NONE as i64
    } else if d <= -HALF {
        d + PCR_NONE as i64
    } else {
        d
    }
}

/// `pcr + ticks` reduced to the PCR ring; either operand may exceed the ring.
pub fn pcr_add(pcr: u64, ticks: u64) -> u64 {
    ((pcr as u128 + ticks as u128) % PCR_NONE as u128) as u64
}

/// `pcr + delta` reduced to the PCR ring for a signed offset; either operand
/// may exceed the ring.
pub fn pcr_add_signed(pcr: u64, delta: i64) -> u64 {
    (pcr as i128 + delta as i128).rem_euclid(PCR_NONE as i128) as u64
}

/// PCR at byte position `pos` of a stream that carries `base` at `pos_base`
/// and runs at `bitrate` bit/s, reduced to the PCR ring. Positions before
/// `pos_base` map to `base`. Panics on `bitrate == 0`.
pub fn pcr_from_pos(base: u64, pos_base: u64, pos: u64, bitrate: u64) -> u64 {
    const TICKS_PER_BYTE_NUM: u128 = 8 * PCR_SYSTEM_CLOCK as u128;
    let ticks = pos.saturating_sub(pos_base) as u128 * TICKS_PER_BYTE_NUM / bitrate as u128;
    ((base as u128 + ticks) % PCR_NONE as u128) as u64
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

    #[test]
    fn test_pcr_signed_delta() {
        assert_eq!(pcr_signed_delta(20000, 30000), 10000);
        assert_eq!(pcr_signed_delta(30000, 20000), -10000);
        assert_eq!(pcr_signed_delta(20000, 20000), 0);

        // across the wrap in both directions
        assert_eq!(pcr_signed_delta(PCR_MAX, 5), 6);
        assert_eq!(pcr_signed_delta(5, PCR_MAX), -6);
        assert_eq!(pcr_signed_delta(PCR_MAX - 5000, 5000), 10001);
        assert_eq!(pcr_signed_delta(5000, PCR_MAX - 5000), -10001);

        // half the ring apart resolves forward from either side
        let half = PCR_NONE / 2;
        assert_eq!(pcr_signed_delta(0, half), half as i64);
        assert_eq!(pcr_signed_delta(half, 0), half as i64);
        assert_eq!(pcr_signed_delta(0, half + 1), -(half as i64) + 1);
        assert_eq!(pcr_signed_delta(0, half - 1), half as i64 - 1);
    }

    #[test]
    fn test_pcr_add() {
        assert_eq!(pcr_add(20000, 10000), 30000);
        assert_eq!(pcr_add(PCR_MAX, 1), 0);
        assert_eq!(pcr_add(PCR_MAX, 6), 5);
        assert_eq!(pcr_add(0, PCR_NONE), 0);
        assert_eq!(pcr_add(PCR_NONE + 7, PCR_NONE + 8), 15);
        let want = ((u64::MAX as u128 * 2) % PCR_NONE as u128) as u64;
        assert_eq!(pcr_add(u64::MAX, u64::MAX), want);
    }

    #[test]
    fn test_pcr_add_signed() {
        assert_eq!(pcr_add_signed(20000, 10000), 30000);
        assert_eq!(pcr_add_signed(20000, -10000), 10000);
        assert_eq!(pcr_add_signed(PCR_MAX, 1), 0);
        assert_eq!(pcr_add_signed(0, -1), PCR_MAX);
        assert_eq!(pcr_add_signed(5, -6), PCR_MAX);
        assert_eq!(pcr_add_signed(0, -(PCR_NONE as i64)), 0);
        assert_eq!(pcr_add_signed(0, -(PCR_NONE as i64) - 1), PCR_MAX);
        let want = (7 + i64::MIN as i128).rem_euclid(PCR_NONE as i128) as u64;
        assert_eq!(pcr_add_signed(PCR_NONE + 7, i64::MIN), want);

        // adding a signed delta and reading it back round-trips
        for from in [0, 12345, PCR_NONE / 2, PCR_MAX] {
            for d in [0, 1, -1, 27_000_000, -27_000_000] {
                assert_eq!(pcr_signed_delta(from, pcr_add_signed(from, d)), d);
            }
        }
    }

    #[test]
    fn test_pcr_from_pos() {
        // 216 Mbit/s = one tick per byte
        const ONE_TICK_PER_BYTE: u64 = 8 * PCR_SYSTEM_CLOCK;
        assert_eq!(pcr_from_pos(1000, 0, 500, ONE_TICK_PER_BYTE), 1500);
        assert_eq!(pcr_from_pos(1000, 200, 700, ONE_TICK_PER_BYTE), 1500);
        assert_eq!(pcr_from_pos(1000, 200, 200, ONE_TICK_PER_BYTE), 1000);
        assert_eq!(pcr_from_pos(1000, 200, 0, ONE_TICK_PER_BYTE), 1000);

        // one second of 1 Mbit/s is 125_000 bytes, truncated to whole ticks
        assert_eq!(pcr_from_pos(0, 0, 125_000, 1_000_000), PCR_SYSTEM_CLOCK);
        assert_eq!(pcr_from_pos(0, 0, 1, 1_000_000), 216);
        assert_eq!(pcr_from_pos(0, 0, 3, 1_000_000_000), 0);

        // the ring is applied to the sum, not to the ticks
        assert_eq!(pcr_from_pos(PCR_MAX, 0, 1, ONE_TICK_PER_BYTE), 0);
        assert_eq!(pcr_from_pos(PCR_MAX, 0, 6, ONE_TICK_PER_BYTE), 5);

        // pos * 216e6 overflows u64 past 85 GB; the math stays in u128
        let pos = 200_000_000_000u64;
        let ticks = pos as u128 * ONE_TICK_PER_BYTE as u128 / 1_000_000;
        let want = (ticks % PCR_NONE as u128) as u64;
        assert_eq!(pcr_from_pos(0, 0, pos, 1_000_000), want);
    }
}
