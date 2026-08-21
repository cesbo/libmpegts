use crate::{
    pcr::{
        PCR_NONE,
        pcr_delta,
    },
    pes::{
        PesHeaderRef,
        Timestamp,
    },
    psi::{
        PAT_PID,
        PatSectionRef,
        PmtSectionMut,
        PmtSectionRef,
        Psi,
    },
    ts::{
        PACKET_SIZE,
        PID_NULL,
        TsPacketMut,
        TsPacketRef,
        build_pcr_packet,
    },
};

// 27 MHz ticks per millisecond
const TICKS_MS: u64 = 27_000;
// 90 kHz units per millisecond
const CLOCK90_MS: u64 = 90;

// Valid stream PCR interval upper bound (27 MHz)
const DELTA_MAX: u64 = 1_000 * TICKS_MS;
// Consecutive reference PCR faults that make the reference unusable
const REF_FAULTS_MAX: u32 = 3;
// Minimum PES pair span that closes a rate sample (90 kHz)
const PES_GRANULARITY: u64 = 300 * CLOCK90_MS;
// Splice threshold for a decode-time step (90 kHz)
const PES_DISCONT: u64 = 1_000 * CLOCK90_MS;
// Consecutive discontinuous probe steps that free the probe PID
const PROBE_FAULTS_MAX: u32 = 3;
// Continuous same-PID PES advance that arms full synthesis in Auto (90 kHz)
const PES_SPARSE_PROBE: u64 = 1_500 * CLOCK90_MS;
// Stream PCR repetition above this is sparse (27 MHz)
const SPARSE_PCR: u64 = 100 * TICKS_MS;
// Sustain window for sparse / dense classification (27 MHz)
const SPARSE_WINDOW: u64 = 2_000 * TICKS_MS;
// Target stream time between emitted PCRs (27 MHz)
const CADENCE_TARGET: u64 = 35 * TICKS_MS;
// Accumulated monotonic-clamp correction that signals a hidden splice
const CLAMP_BUDGET: u64 = 100 * TICKS_MS;
// Decaying rate window span (27 MHz); bytes and ticks halve when exceeded
const RATE_WINDOW: u64 = 2_000 * TICKS_MS;
// Bytes of stream without any PMT before the timing PID becomes the carrier
const PMT_WAIT: u64 = 2 * 1024 * 1024;
// No top-up injection inside this margin before the predicted next real PCR
const STOP_WINDOW: u64 = 20 * TICKS_MS;
// Timing step magnitude treated as a splice right after a segment boundary
const SEGMENT_SPLICE: u64 = 200 * CLOCK90_MS;
// Default PCR lead behind the decode timestamp
const DEFAULT_DELAY_VIDEO_MS: u64 = 700;
const DEFAULT_DELAY_OTHER_MS: u64 = 150;

const HALF_PCR: u64 = PCR_NONE / 2;

/// Operating mode for [`PcrSynth`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcrSynthMode {
    /// Detect whether the stream needs synthesis and arm it automatically.
    Auto,
    /// Full synthesis from the first anchor and rate, without probation.
    Force,
}

/// Configuration for [`PcrSynth`].
#[derive(Debug, Clone)]
pub struct PcrSynthConfig {
    /// Operating mode.
    pub mode: PcrSynthMode,
    /// PCR lead (delay behind DTS) override in ms. Default when `None`:
    /// 700 ms if the timing PID is video, 150 ms otherwise.
    pub pcr_delay_ms: Option<u64>,
}

impl Default for PcrSynthConfig {
    fn default() -> Self {
        Self {
            mode: PcrSynthMode::Auto,
            pcr_delay_ms: None,
        }
    }
}

/// Current stage of [`PcrSynth`] operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcrSynthPhase {
    /// Watching the stream; nothing decided yet, packets pass verbatim.
    Probing,
    /// Stream PCR is healthy; stage is transparent.
    Passive,
    /// Full synthesis: PES-derived clock, injection + restamp of existing PCRs.
    Full,
    /// Valid but sparse stream PCR; interpolated top-up injection between real PCRs.
    TopUp,
    /// No usable PCR and no usable PES timestamps; transparent.
    NoTiming,
    /// More than one program in PAT; stage disabled, transparent.
    MultiProgram,
}

/// Snapshot of [`PcrSynth`] state and counters.
#[derive(Debug, Clone)]
pub struct PcrSynthStatus {
    /// Current phase.
    pub phase: PcrSynthPhase,
    /// PID that carries emitted PCR.
    pub carrier_pid: Option<u16>,
    /// PES PID elected to drive the synthetic clock; set in any phase.
    pub timing_pid: Option<u16>,
    /// `PCR_PID=0x1FFF` PMT rewrite active.
    pub pmt_patched: bool,
    /// AF-only PCR packets injected.
    pub injected: u64,
    /// Existing PCR fields rewritten.
    pub restamped: u64,
    /// DI re-anchors emitted.
    pub discontinuities: u64,
}

/// Decaying byte-rate window: bytes observed against 27 MHz stream ticks.
#[derive(Debug, Default)]
struct RateWindow {
    bytes: u64,
    ticks: u64,
}

impl RateWindow {
    fn push(&mut self, bytes: u64, ticks: u64) {
        self.bytes = self.bytes.saturating_add(bytes);
        self.ticks = self.ticks.saturating_add(ticks);
        while self.ticks > RATE_WINDOW && self.bytes > 0 {
            self.bytes /= 2;
            self.ticks /= 2;
        }
    }

    fn is_ready(&self) -> bool {
        self.bytes > 0 && self.ticks > 0
    }

    /// Stream time in 27 MHz ticks covered by `bytes` at the measured rate.
    fn ticks_for(&self, bytes: u64) -> u64 {
        if self.bytes == 0 {
            return 0;
        }
        ((bytes as u128 * self.ticks as u128) / self.bytes as u128) as u64
    }
}

/// Outcome of a timing PID observation.
#[derive(Debug, Clone, Copy)]
enum TimingStep {
    /// No timeline change (no timestamp, or a presentation reorder).
    Idle,
    /// Timeline advanced normally.
    Advance,
    /// Decode-time discontinuity: the timeline was re-anchored.
    Splice,
}

/// PES-derived clock: timing PID election, anchor and byte rate.
#[derive(Debug, Default)]
struct Clock {
    timing_pid: Option<u16>,
    timing_is_video: bool,
    candidate_pid: Option<u16>,
    last_ts: Option<Timestamp>,
    pair: Option<(Timestamp, u64)>,
    anchor: Option<(Timestamp, u64)>,
    rate: RateWindow,
    seg_pending: bool,
}

impl Clock {
    /// Commits the timing PID. Video commits on first sighting, a non-video
    /// candidate on its second, so a video PID seen in between wins.
    /// Returns `true` when `pid` is the committed timing PID.
    fn elect(&mut self, pid: u16, stream_id: u8) -> bool {
        if let Some(timing) = self.timing_pid {
            return timing == pid;
        }

        if (0xE0 ..= 0xEF).contains(&stream_id) {
            self.timing_pid = Some(pid);
            self.timing_is_video = true;
            self.candidate_pid = None;
            return true;
        }

        if self.candidate_pid == Some(pid) {
            self.timing_pid = Some(pid);
            self.timing_is_video = false;
            return true;
        }

        if self.candidate_pid.is_none() {
            self.candidate_pid = Some(pid);
        }
        false
    }

    /// Feeds a decode-order timestamp seen at byte position `pos`.
    fn observe(&mut self, ts: Timestamp, pos: u64) -> TimingStep {
        let Some(last) = self.last_ts else {
            self.seg_pending = false;
            self.reanchor(ts, pos);
            return TimingStep::Advance;
        };

        let backward = ts.is_before(last);
        let magnitude = if backward {
            last.wrapping_sub(ts).value()
        } else {
            ts.wrapping_sub(last).value()
        };

        if self.seg_pending {
            self.seg_pending = false;
            if magnitude >= SEGMENT_SPLICE {
                self.reanchor(ts, pos);
                return TimingStep::Splice;
            }
        }

        if magnitude >= PES_DISCONT {
            self.reanchor(ts, pos);
            return TimingStep::Splice;
        }

        if backward {
            // Presentation reorder
            return TimingStep::Idle;
        }

        self.last_ts = Some(ts);

        if let Some((open_ts, open_pos)) = self.pair {
            let span = ts.wrapping_sub(open_ts).value();
            if span >= PES_GRANULARITY {
                self.rate.push(pos.saturating_sub(open_pos), span * 300);
                self.pair = Some((ts, pos));
            }
        } else {
            self.pair = Some((ts, pos));
        }

        self.anchor = Some((ts, pos));
        TimingStep::Advance
    }

    /// Restarts the timeline at (`ts`, `pos`); the rate window is kept.
    fn reanchor(&mut self, ts: Timestamp, pos: u64) {
        self.last_ts = Some(ts);
        self.pair = Some((ts, pos));
        self.anchor = Some((ts, pos));
    }
}

/// Per-packet pass-through stage that guarantees a valid PCR in an SPTS
/// stream. One instance covers one connection and one timeline; the caller
/// rebuilds the instance per connection.
///
/// [`process`](Self::process) never reorders or loses input packets: every
/// input packet is emitted exactly once (possibly modified), in order;
/// injections are extra packets.
///
/// # Example
///
/// ```
/// use libmpegts::pcr::{PcrSynthConfig, PcrSynthPhase, PcrSynth};
/// use libmpegts::ts::NULL_PACKET;
///
/// let mut synth = PcrSynth::new(PcrSynthConfig::default());
///
/// let mut out = Vec::new();
/// synth.process(NULL_PACKET.as_ref(), &mut out);
///
/// // Nothing is decided yet: the packet passes through verbatim
/// assert_eq!(out, NULL_PACKET.as_ref());
/// assert_eq!(synth.status().phase, PcrSynthPhase::Probing);
/// ```
pub struct PcrSynth {
    config: PcrSynthConfig,
    bytes_seen: u64,

    // PSI
    pat_psi: Psi,
    pmt_psi: Psi,
    multiprogram: bool,
    pmt_pid: Option<u16>,
    pmt_pcr_pid: Option<u16>,
    pmt_version: u8,
    pmt_cache: Vec<u8>,
    pmt_cache_crc: Option<u32>,
    pmt_patch: Vec<u8>,
    pmt_flow: Option<usize>,

    // reference PCR tracking
    ref_pid: Option<u16>,
    ref_last: Option<(u64, u64)>,
    ref_faults: u32,
    ref_ever_valid: bool,
    ref_interval: Option<u64>,
    ref_rate: RateWindow,
    sparse_run: u64,
    dense_run: u64,
    topup: bool,

    // PES probe (Auto arming)
    probe: Option<(u16, Timestamp, Timestamp)>,
    probe_faults: u32,

    // clock and emission
    clock: Clock,
    full_armed: bool,
    carrier: Option<u16>,
    cc_by_pid: [u8; PID_NULL as usize + 1],
    last_emitted: Option<u64>,
    clamp_debt: u64,

    // counters
    injected: u64,
    restamped: u64,
    discontinuities: u64,
}

impl PcrSynth {
    /// Creates a synthesizer with the given configuration.
    pub fn new(config: PcrSynthConfig) -> Self {
        let full_armed = matches!(config.mode, PcrSynthMode::Force);
        Self {
            config,
            bytes_seen: 0,

            pat_psi: Psi::default(),
            pmt_psi: Psi::default(),
            multiprogram: false,
            pmt_pid: None,
            pmt_pcr_pid: None,
            pmt_version: 0,
            pmt_cache: Vec::new(),
            pmt_cache_crc: None,
            pmt_patch: Vec::new(),
            pmt_flow: None,

            ref_pid: None,
            ref_last: None,
            ref_faults: 0,
            ref_ever_valid: false,
            ref_interval: None,
            ref_rate: RateWindow::default(),
            sparse_run: 0,
            dense_run: 0,
            topup: false,

            probe: None,
            probe_faults: 0,

            clock: Clock::default(),
            full_armed,
            carrier: None,
            cc_by_pid: [0u8; PID_NULL as usize + 1],
            last_emitted: None,
            clamp_debt: 0,

            injected: 0,
            restamped: 0,
            discontinuities: 0,
        }
    }

    /// Processes one TS packet. Appends zero or more whole 188-byte packets
    /// to `out`: usually the input packet itself (possibly with a restamped
    /// PCR or patched PMT bytes), preceded by an injected AF-only PCR packet
    /// when one is due.
    pub fn process(&mut self, packet: &[u8; PACKET_SIZE], out: &mut Vec<u8>) {
        let pos = self.bytes_seen;
        self.bytes_seen = self.bytes_seen.saturating_add(PACKET_SIZE as u64);

        let ts = TsPacketRef::from(packet);
        if !ts.is_sync() || ts.is_error() {
            out.extend_from_slice(packet);
            return;
        }

        let pid = ts.pid();

        if pid == PAT_PID {
            self.on_pat(packet);
        }

        if self.multiprogram {
            out.extend_from_slice(packet);
            return;
        }

        let is_pmt = pid != PAT_PID && self.pmt_pid == Some(pid);
        let mut pmt_fresh = false;
        if is_pmt {
            pmt_fresh = self.on_pmt_learn(packet);
        }

        let af = ts.adaptation_field();
        let input_pcr = af.as_ref().and_then(|af| af.pcr());
        let input_di = af.as_ref().is_some_and(|af| af.discontinuity_indicator());
        if let Some(pcr) = input_pcr
            && pid != PID_NULL
        {
            self.on_input_pcr(pid, pcr, input_di, pos);
        }

        let mut step = TimingStep::Idle;
        if !is_pmt
            && pid != PAT_PID
            && pid != PID_NULL
            && ts.is_payload_start()
            && ts.scrambling_control() == 0
            && let Some(payload) = ts.payload()
            && let Ok(header) = PesHeaderRef::try_from(payload)
            && let Some(pts_dts) = header.pts_dts()
        {
            let t = pts_dts.timestamp();
            self.probe_observe(pid, t);
            if self.clock.elect(pid, header.stream_id()) {
                step = self.clock.observe(t, pos);
            }
        }

        if !self.full_armed
            && self.pmt_pcr_pid == Some(PID_NULL)
            && self.clock.timing_pid.is_some()
            && self.clock.rate.is_ready()
        {
            self.arm_full();
        }

        let mut buf = *packet;
        if self.full_armed {
            self.elect_carrier();
            self.full_flow(&mut buf, pos, step, input_pcr.is_some(), out);
            if is_pmt && self.patch_mode() {
                self.pmt_stream_patch(&mut buf, pmt_fresh);
            }
        } else if self.topup {
            self.topup_flow(pid, pos, input_pcr, out);
        }
        out.extend_from_slice(&buf);

        // AF-only injections repeat the last payload CC on their PID
        if ts.payload().is_some() {
            self.cc_by_pid[(pid & PID_NULL) as usize] = ts.cc();
        }
    }

    /// Hint: a container boundary (e.g. an HLS segment) was crossed before
    /// the next packet. The next timing step of 200 ms or more is a splice;
    /// a smaller step clears the hint.
    pub fn segment_boundary(&mut self) {
        self.clock.seg_pending = true;
    }

    /// Returns a snapshot of the current phase and counters.
    pub fn status(&self) -> PcrSynthStatus {
        let carrier_pid = if self.full_armed {
            self.carrier
        } else if self.topup {
            self.ref_pid
        } else {
            None
        };

        PcrSynthStatus {
            phase: self.phase(),
            carrier_pid,
            timing_pid: self.clock.timing_pid,
            pmt_patched: self.patch_mode(),
            injected: self.injected,
            restamped: self.restamped,
            discontinuities: self.discontinuities,
        }
    }

    fn phase(&self) -> PcrSynthPhase {
        if self.multiprogram {
            PcrSynthPhase::MultiProgram
        } else if self.full_armed {
            PcrSynthPhase::Full
        } else if self.topup {
            PcrSynthPhase::TopUp
        } else if self.ref_ever_valid && self.ref_faults < REF_FAULTS_MAX {
            PcrSynthPhase::Passive
        } else if self.bytes_seen >= PMT_WAIT
            && !self.ref_ever_valid
            && !self.clock.rate.is_ready()
        {
            PcrSynthPhase::NoTiming
        } else {
            PcrSynthPhase::Probing
        }
    }

    fn arm_full(&mut self) {
        self.full_armed = true;
        self.topup = false;
    }

    /// PCR lead in 27 MHz ticks.
    fn lead(&self) -> u64 {
        let ms = self.config.pcr_delay_ms.unwrap_or({
            if self.clock.timing_is_video {
                DEFAULT_DELAY_VIDEO_MS
            } else {
                DEFAULT_DELAY_OTHER_MS
            }
        });
        ms.saturating_mul(TICKS_MS) % PCR_NONE
    }

    /// Synthetic timeline value at byte position `pos`.
    fn pcr_at(&self, pos: u64) -> Option<u64> {
        let (anchor_ts, anchor_pos) = self.clock.anchor?;
        if !self.clock.rate.is_ready() {
            return None;
        }

        let ticks = self.clock.rate.ticks_for(pos.saturating_sub(anchor_pos));
        let base = (anchor_ts.value() as u128 * 300 + ticks as u128) % PCR_NONE as u128;
        let value = (base + PCR_NONE as u128 - self.lead() as u128) % PCR_NONE as u128;
        Some(value as u64)
    }

    /// Applies the per-era monotonic clamp to a candidate value.
    /// Returns the value to emit and whether the clamp budget was exhausted
    /// (hidden splice: the raw candidate is emitted with a DI re-anchor).
    fn clamp(&mut self, candidate: u64) -> (u64, bool) {
        let Some(last) = self.last_emitted else {
            return (candidate, false);
        };

        let forward = pcr_delta(last, candidate);
        if forward != 0 && forward <= HALF_PCR {
            self.clamp_debt = 0;
            return (candidate, false);
        }

        let clamped = (last + 1) % PCR_NONE;
        let correction = pcr_delta(candidate, clamped);
        self.clamp_debt = self.clamp_debt.saturating_add(correction);

        if self.clamp_debt > CLAMP_BUDGET {
            self.clamp_debt = 0;
            return (candidate, true);
        }

        (clamped, false)
    }

    /// Emits an AF-only PCR packet on `pid` before the current input packet.
    fn inject(&mut self, out: &mut Vec<u8>, pid: u16, value: u64, di: bool) {
        let mut packet = [0u8; PACKET_SIZE];
        let cc = self.cc_by_pid[(pid & PID_NULL) as usize];
        build_pcr_packet(&mut packet, pid, cc, value, di);
        out.extend_from_slice(&packet);

        self.last_emitted = Some(value);
        self.injected += 1;
        if di {
            self.discontinuities += 1;
        }
    }

    /// Full-synthesis path: splice signaling, cadence injection and the
    /// ownership restamp of existing PCR fields.
    fn full_flow(
        &mut self,
        buf: &mut [u8; PACKET_SIZE],
        pos: u64,
        step: TimingStep,
        has_pcr: bool,
        out: &mut Vec<u8>,
    ) {
        let ready = self.carrier.is_some()
            && self.clock.anchor.is_some()
            && self.clock.rate.is_ready();

        if ready {
            if matches!(step, TimingStep::Splice) {
                // The DI-flagged re-anchor precedes the triggering packet
                if let (Some(candidate), Some(carrier)) = (self.pcr_at(pos), self.carrier) {
                    self.clamp_debt = 0;
                    self.last_emitted = None;
                    self.inject(out, carrier, candidate, true);
                }
            }

            if has_pcr {
                if let Some(candidate) = self.pcr_at(pos) {
                    let (value, forced_di) = self.clamp(candidate);
                    let mut packet = TsPacketMut::from(&mut *buf);
                    packet.set_pcr(value);
                    if forced_di {
                        packet.set_discontinuity();
                        self.discontinuities += 1;
                    }
                    self.last_emitted = Some(value);
                    self.restamped += 1;
                }
            } else {
                let due = match self.last_emitted {
                    None => true,
                    Some(last) => self
                        .pcr_at(pos + PACKET_SIZE as u64)
                        .is_some_and(|at_end| pcr_delta(last, at_end) > CADENCE_TARGET),
                };
                if due
                    && let (Some(candidate), Some(carrier)) = (self.pcr_at(pos), self.carrier)
                {
                    let (value, forced_di) = self.clamp(candidate);
                    self.inject(out, carrier, value, forced_di);
                }
            }
        }
    }

    /// Top-up path: interpolated injection between real reference PCRs.
    fn topup_flow(&mut self, pid: u16, pos: u64, input_pcr: Option<u64>, out: &mut Vec<u8>) {
        if input_pcr.is_some() && Some(pid) == self.ref_pid {
            // Real reference PCRs pass verbatim and re-anchor the interpolation
            self.last_emitted = input_pcr;
            return;
        }

        let Some(last_emitted) = self.last_emitted else {
            return;
        };
        let (Some((last_real, last_real_pos)), Some(interval)) =
            (self.ref_last, self.ref_interval)
        else {
            return;
        };
        if !self.ref_rate.is_ready() {
            return;
        }

        let end_bytes = (pos + PACKET_SIZE as u64).saturating_sub(last_real_pos);
        let at_end = (last_real + self.ref_rate.ticks_for(end_bytes)) % PCR_NONE;
        if pcr_delta(last_emitted, at_end) <= CADENCE_TARGET {
            return;
        }

        let value =
            (last_real + self.ref_rate.ticks_for(pos.saturating_sub(last_real_pos))) % PCR_NONE;
        let predicted = (last_real + interval) % PCR_NONE;

        // Clamp to the stop window edge before the predicted next real PCR,
        // so an on-time real PCR never steps backward
        let margin = pcr_delta(value, predicted);
        let value = if margin <= STOP_WINDOW || margin > HALF_PCR {
            (predicted + PCR_NONE - STOP_WINDOW) % PCR_NONE
        } else {
            value
        };

        let forward = pcr_delta(last_emitted, value);
        if forward == 0 || forward > HALF_PCR {
            return;
        }

        if let Some(carrier) = self.ref_pid {
            self.inject(out, carrier, value, false);
        }
    }

    /// Tracks the reference PCR PID: validity, faults, sparse repetition.
    fn on_input_pcr(&mut self, pid: u16, pcr: u64, di: bool, pos: u64) {
        let Some(ref_pid) = self.ref_pid else {
            self.ref_pid = Some(pid);
            self.ref_last = Some((pcr, pos));
            self.probe_hold();
            return;
        };

        if ref_pid != pid {
            // Foreign-PID PCRs hold the probe while the reference is usable
            if self.ref_faults < REF_FAULTS_MAX {
                self.probe_hold();
            }
            return;
        }

        let Some((last, last_pos)) = self.ref_last else {
            self.ref_last = Some((pcr, pos));
            return;
        };

        let delta = pcr_delta(last, pcr);
        self.ref_last = Some((pcr, pos));

        if di {
            // A DI-flagged jump is a legitimate discontinuity, not a fault
            if self.ref_faults < REF_FAULTS_MAX {
                self.probe_hold();
            }
            return;
        }

        if delta > 0 && delta <= DELTA_MAX {
            self.ref_faults = 0;
            self.ref_ever_valid = true;
            self.ref_interval = Some(delta);
            self.ref_rate.push(pos.saturating_sub(last_pos), delta);
            self.probe_hold();

            if delta > SPARSE_PCR {
                self.sparse_run = self.sparse_run.saturating_add(delta);
                self.dense_run = 0;
                if !self.full_armed && !self.topup && self.sparse_run >= SPARSE_WINDOW {
                    self.topup = true;
                }
            } else {
                self.dense_run = self.dense_run.saturating_add(delta);
                self.sparse_run = 0;
                if self.topup && self.dense_run >= SPARSE_WINDOW {
                    self.topup = false;
                }
            }
            return;
        }

        self.ref_faults = self.ref_faults.saturating_add(1);
        if self.ref_faults >= REF_FAULTS_MAX {
            // An unusable reference stops holding the probe
            self.topup = false;
        } else {
            self.probe_hold();
        }
    }

    /// PES probe: continuous same-PID decode-time advance arms Full (Auto).
    fn probe_observe(&mut self, pid: u16, ts: Timestamp) {
        if self.full_armed {
            return;
        }

        let Some((probe_pid, start, cur)) = self.probe else {
            self.probe = Some((pid, ts, ts));
            return;
        };

        if probe_pid != pid {
            return;
        }

        let backward = ts.is_before(cur);
        let magnitude = if backward {
            cur.wrapping_sub(ts).value()
        } else {
            ts.wrapping_sub(cur).value()
        };

        if magnitude >= PES_DISCONT {
            self.probe_faults += 1;
            if self.probe_faults >= PROBE_FAULTS_MAX {
                self.probe = None;
                self.probe_faults = 0;
            } else {
                self.probe = Some((probe_pid, ts, ts));
            }
            return;
        }

        if backward {
            // Presentation reorder
            return;
        }

        self.probe = Some((probe_pid, start, ts));
        if ts.wrapping_sub(start).value() >= PES_SPARSE_PROBE {
            self.arm_full();
        }
    }

    /// A usable stream PCR holds the probe: continuous advance restarts.
    fn probe_hold(&mut self) {
        if let Some((pid, _start, cur)) = self.probe {
            self.probe = Some((pid, cur, cur));
        }
    }

    /// Elects the PID that carries emitted PCR in full synthesis.
    fn elect_carrier(&mut self) {
        let desired = match self.pmt_pcr_pid {
            Some(PID_NULL) => self.clock.timing_pid,
            Some(pid) => Some(pid),
            None if self.bytes_seen >= PMT_WAIT => self.clock.timing_pid,
            None => None,
        };

        if desired.is_some() && desired != self.carrier {
            self.carrier = desired;
            self.rebuild_pmt_patch();
        }
    }

    /// The streaming PMT rewrite is active: full synthesis with an upstream
    /// `PCR_PID` of `0x1FFF` and an elected carrier.
    fn patch_mode(&self) -> bool {
        self.full_armed && self.pmt_pcr_pid == Some(PID_NULL) && self.carrier.is_some()
    }

    /// Parses the PAT: program count gates the stage, a single program names
    /// the PMT PID to follow. The PAT is never modified.
    fn on_pat(&mut self, packet: &[u8; PACKET_SIZE]) {
        let ts = TsPacketRef::from(packet);

        let mut programs = 0usize;
        let mut pmt_pid = None;

        {
            let Some(section) = self.pat_psi.assemble(&ts) else {
                return;
            };
            let Ok(pat) = PatSectionRef::try_from(section) else {
                return;
            };
            for program in pat.programs() {
                let Ok(program) = program else {
                    return;
                };
                if program.program_number() != 0 {
                    programs += 1;
                    pmt_pid = Some(program.pid());
                }
            }
        }

        self.multiprogram = programs > 1;

        let pmt_pid = if programs == 1 { pmt_pid } else { None };
        if self.pmt_pid != pmt_pid {
            self.pmt_pid = pmt_pid;
            self.pmt_psi.clear();
            self.pmt_pcr_pid = None;
            self.pmt_cache.clear();
            self.pmt_cache_crc = None;
            self.pmt_patch.clear();
            self.pmt_flow = None;
        }
    }

    /// Learns the declared `PCR_PID` and caches the section bytes for the
    /// streaming patch. Returns `true` when a section unseen before (by CRC)
    /// completed on this packet.
    fn on_pmt_learn(&mut self, packet: &[u8; PACKET_SIZE]) -> bool {
        let ts = TsPacketRef::from(packet);

        let mut learned: Option<(u16, u8, u32, Vec<u8>)> = None;
        if let Some(section) = self.pmt_psi.assemble(&ts)
            && let Ok(pmt) = PmtSectionRef::try_from(section)
        {
            let crc = pmt.crc32();
            let bytes = if self.pmt_cache_crc == Some(crc) {
                Vec::new()
            } else {
                section.to_vec()
            };
            learned = Some((pmt.pcr_pid(), pmt.version(), crc, bytes));
        }

        let Some((pcr_pid, version, crc, bytes)) = learned else {
            return false;
        };

        self.pmt_pcr_pid = Some(pcr_pid);
        self.pmt_version = version;

        if self.pmt_cache_crc != Some(crc) {
            self.pmt_cache_crc = Some(crc);
            self.pmt_cache = bytes;
            self.rebuild_pmt_patch();
            return true;
        }
        false
    }

    /// Precomputes the patched copy of the cached PMT section: `PCR_PID` set
    /// to the carrier, version bumped, CRC32 recomputed.
    fn rebuild_pmt_patch(&mut self) {
        self.pmt_patch.clear();
        self.pmt_flow = None;

        if !self.patch_mode() || self.pmt_cache.is_empty() {
            return;
        }
        let Some(carrier) = self.carrier else {
            return;
        };

        let mut patched = self.pmt_cache.clone();
        let version = self.pmt_version.wrapping_add(1) & 0x1F;
        if let Ok(mut pmt) = PmtSectionMut::try_from(&mut patched[..]) {
            pmt.set_pcr_pid(carrier);
            pmt.set_version(version);
            pmt.update_crc32();
            self.pmt_patch = patched;
        }
    }

    /// Rewrites one region of a flowing PMT packet from the patched copy
    /// when it matches the cached section; `false` on mismatch.
    fn patch_region(
        &self,
        buf: &mut [u8; PACKET_SIZE],
        pkt: usize,
        sec: usize,
        len: usize,
    ) -> bool {
        if buf[pkt .. pkt + len] != self.pmt_cache[sec .. sec + len] {
            return false;
        }
        buf[pkt .. pkt + len].copy_from_slice(&self.pmt_patch[sec .. sec + len]);
        true
    }

    /// Patches a flowing PMT packet in place: bytes that match the cached
    /// section are rewritten from the patched copy, unknown bytes pass
    /// verbatim. `fresh` marks the packet that completed the first sighting
    /// of a new section: that sighting passes verbatim.
    fn pmt_stream_patch(&mut self, buf: &mut [u8; PACKET_SIZE], fresh: bool) {
        if self.pmt_patch.is_empty() || self.pmt_patch.len() != self.pmt_cache.len() {
            self.pmt_flow = None;
            return;
        }

        let (pusi, payload_offset, payload_len) = {
            let ts = TsPacketRef::from(&*buf);
            let Some(payload) = ts.payload() else {
                self.pmt_flow = None;
                return;
            };
            (ts.is_payload_start(), PACKET_SIZE - payload.len(), payload.len())
        };

        let section_len = self.pmt_cache.len();

        if !pusi {
            let Some(offset) = self.pmt_flow else {
                return;
            };
            let n = payload_len.min(section_len.saturating_sub(offset));
            if n == 0 || !self.patch_region(buf, payload_offset, offset, n) {
                // Upstream changed the PMT: pass verbatim and re-learn
                self.pmt_flow = None;
                return;
            }
            self.pmt_flow = (offset + n < section_len).then_some(offset + n);
            return;
        }

        let pointer = buf[payload_offset] as usize;
        if 1 + pointer > payload_len {
            self.pmt_flow = None;
            return;
        }

        if let Some(offset) = self.pmt_flow {
            // A matching tail is patched even when the rest of the packet
            // differs, so a repetition whose head was already patched
            // completes with a valid CRC
            let n = pointer.min(section_len.saturating_sub(offset));
            if n > 0 {
                self.patch_region(buf, payload_offset + 1, offset, n);
            }
        }
        self.pmt_flow = None;

        let mut start = payload_offset + 1 + pointer;

        if fresh && PACKET_SIZE - start >= section_len {
            return;
        }

        // A short section may repeat back-to-back within one packet
        while start < PACKET_SIZE && buf[start] != 0xFF {
            let n = (PACKET_SIZE - start).min(section_len);
            if !self.patch_region(buf, start, 0, n) {
                // Upstream changed the PMT: pass verbatim and re-learn
                return;
            }
            if n < section_len {
                self.pmt_flow = Some(n);
                return;
            }
            start += n;
        }
    }
}

#[cfg(test)]
mod tests;
