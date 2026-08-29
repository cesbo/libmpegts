use std::collections::HashMap;

use crate::{
    pes::{
        EsFrame,
        PesHeader,
        PesHeaderRef,
        PesPacketizer,
        PtsDts,
        STREAM_ID_AUDIO,
        STREAM_ID_VIDEO,
    },
    psi::{
        PatBuilder,
        PatConfig,
        PatProgram,
        PmtBuilder,
        PmtConfig,
        PmtSectionMut,
        PmtSectionRef,
        PmtStream,
        Psi,
        PsiPacketizer,
        Sections,
    },
    ts::{
        self,
        PACKET_SIZE,
        TsPacketMut,
        TsPacketRef,
    },
};

use super::*;

const VIDEO_PID: u16 = 256;
const AUDIO_PID: u16 = 257;
const PMT_PID: u16 = 4096;
const MASK33: u64 = (1 << 33) - 1;
const TICKS_MS: u64 = 27_000;
const LEAD_VIDEO: u64 = 700 * TICKS_MS;
const LEAD_OTHER: u64 = 150 * TICKS_MS;

fn auto() -> PcrSynthConfig {
    PcrSynthConfig::default()
}

fn expected_pcr(ts90: u64, lead: u64) -> u64 {
    ((ts90 & MASK33) * 300 + PCR_NONE - lead) % PCR_NONE
}

/// Wrap-aware absolute distance between two PCR values.
fn pcr_dist(a: u64, b: u64) -> u64 {
    pcr_delta(a, b).min(pcr_delta(b, a))
}

struct Fixture {
    packets: Vec<[u8; PACKET_SIZE]>,
    video: PesPacketizer,
    audio: PesPacketizer,
    pat: PsiPacketizer,
    pmt: PsiPacketizer,
    last_cc: HashMap<u16, u8>,
}

fn pmt_sections(pcr_pid: u16, version: u8, video_desc_len: usize) -> Sections {
    let video_desc = if video_desc_len > 0 {
        let mut d = vec![0u8; video_desc_len];
        d[0] = 0x05;
        d[1] = (video_desc_len - 2) as u8;
        d
    } else {
        Vec::new()
    };

    PmtBuilder::build(PmtConfig {
        program_number: 1,
        pcr_pid,
        version,
        program_descriptors: Vec::new(),
        streams: vec![
            PmtStream {
                stream_type: 0x1B,
                elementary_pid: VIDEO_PID,
                stream_descriptors: video_desc,
            },
            PmtStream {
                stream_type: 0x0F,
                elementary_pid: AUDIO_PID,
                stream_descriptors: Vec::new(),
            },
        ],
    })
}

impl Fixture {
    fn new(pmt_pcr_pid: u16) -> Self {
        let mut pat = PsiPacketizer::new(0);
        pat.set_sections(PatBuilder::build(PatConfig {
            transport_stream_id: 1,
            version: 0,
            programs: vec![PatProgram {
                program_number: 1,
                pid: PMT_PID,
            }],
        }));

        let mut pmt = PsiPacketizer::new(PMT_PID);
        pmt.set_sections(pmt_sections(pmt_pcr_pid, 1, 0));

        Self {
            packets: Vec::new(),
            video: PesPacketizer::new(VIDEO_PID),
            audio: PesPacketizer::new(AUDIO_PID),
            pat,
            pmt,
            last_cc: HashMap::new(),
        }
    }

    fn push(&mut self, packet: [u8; PACKET_SIZE]) {
        if (packet[3] & 0x10) != 0 {
            let pid = (u16::from(packet[1] & 0x1F) << 8) | u16::from(packet[2]);
            self.last_cc.insert(pid, packet[3] & 0x0F);
        }
        self.packets.push(packet);
    }

    fn set_pmt(&mut self, pcr_pid: u16, version: u8, video_desc_len: usize) {
        self.pmt.set_sections(pmt_sections(pcr_pid, version, video_desc_len));
    }

    fn psi(&mut self) {
        let mut packet = [0u8; PACKET_SIZE];
        self.pat.reset();
        while self.pat.next(&mut packet) {
            self.push(packet);
        }
        self.pmt.reset();
        while self.pmt.next(&mut packet) {
            self.push(packet);
        }
    }

    fn video_frame(&mut self, pts: u64, dts: Option<u64>, len: usize) {
        let pts_dts = match dts {
            Some(dts) => PtsDts::new(pts).with_dts(dts),
            None => PtsDts::new(pts),
        };
        let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(pts_dts);
        self.video.set_frame(EsFrame {
            header,
            payload: vec![0u8; len],
            rai: false,
        });
        let mut packet = [0u8; PACKET_SIZE];
        while self.video.next(&mut packet) {
            self.push(packet);
        }
    }

    fn scrambled_video_frame(&mut self, pts: u64, len: usize) {
        let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(PtsDts::new(pts));
        self.video.set_frame(EsFrame {
            header,
            payload: vec![0u8; len],
            rai: false,
        });
        let mut packet = [0u8; PACKET_SIZE];
        while self.video.next(&mut packet) {
            packet[3] |= 0x80; // transport_scrambling_control
            self.push(packet);
        }
    }

    fn audio_frame(&mut self, pts: u64, len: usize) {
        let header = PesHeader::new(STREAM_ID_AUDIO).with_pts_dts(PtsDts::new(pts));
        self.audio.set_frame(EsFrame {
            header,
            payload: vec![0u8; len],
            rai: false,
        });
        let mut packet = [0u8; PACKET_SIZE];
        while self.audio.next(&mut packet) {
            self.push(packet);
        }
    }

    /// AF-only PCR packet with the CC of the last payload packet on `pid`.
    fn pcr_packet(&mut self, pid: u16, pcr: u64, di: bool) {
        let cc = self.last_cc.get(&pid).copied().unwrap_or(0);
        let mut packet = [0u8; PACKET_SIZE];
        ts::build_pcr_packet(&mut packet, pid, cc, pcr, di);
        self.packets.push(packet);
    }

    fn null_packet(&mut self) {
        self.packets.push(*ts::NULL_PACKET.as_ref());
    }

    /// Standard A/V frame: 25 fps video with PTS+DTS, audio on even frames,
    /// PSI every 12 frames.
    fn av_frame(&mut self, i: u64, dts_start: u64) {
        let dts = dts_start.wrapping_add(i * 3600) & MASK33;
        if i.is_multiple_of(12) {
            self.psi();
        }
        self.video_frame((dts + 7200) & MASK33, Some(dts), 700);
        if i.is_multiple_of(2) {
            self.audio_frame((dts + 7200) & MASK33, 200);
        }
    }
}

fn run(synth: &mut PcrSynth, packets: &[[u8; PACKET_SIZE]]) -> Vec<u8> {
    let mut out = Vec::new();
    for packet in packets {
        synth.process(packet, &mut out);
    }
    out
}

#[derive(Debug, Clone, Copy)]
struct Event {
    index: usize,
    pid: u16,
    pcr: Option<u64>,
    di: bool,
    cc: u8,
    af_len: Option<u8>,
    has_payload: bool,
    tsc: u8,
    dts: Option<u64>,
}

fn events(out: &[u8]) -> Vec<Event> {
    assert!(out.len().is_multiple_of(PACKET_SIZE), "output not packet-aligned");
    out.chunks_exact(PACKET_SIZE)
        .enumerate()
        .map(|(index, chunk)| {
            let arr: &[u8; PACKET_SIZE] = chunk.try_into().unwrap();
            let ts = TsPacketRef::from(arr);
            let af = ts.adaptation_field();
            let pcr = af.as_ref().and_then(|af| af.pcr());
            let di = af.as_ref().is_some_and(|af| af.discontinuity_indicator());
            let af_len = ((arr[3] & 0x20) != 0).then(|| arr[4]);
            let has_payload = (arr[3] & 0x10) != 0;

            let mut dts = None;
            if ts.is_payload_start()
                && ts.scrambling_control() == 0
                && let Some(payload) = ts.payload()
                && let Ok(header) = PesHeaderRef::try_from(payload)
            {
                dts = header.pts_dts().map(|p| p.timestamp().value());
            }

            Event {
                index,
                pid: ts.pid(),
                pcr,
                di,
                cc: ts.cc(),
                af_len,
                has_payload,
                tsc: ts.scrambling_control(),
                dts,
            }
        })
        .collect()
}

fn pcr_events(events: &[Event], pid: u16) -> Vec<Event> {
    events
        .iter()
        .filter(|e| e.pid == pid && e.pcr.is_some())
        .copied()
        .collect()
}

/// PCR values on `pid` are strictly monotone wrap-aware within eras.
fn assert_monotone(events: &[Event], pid: u16) {
    let mut last: Option<u64> = None;
    for e in events.iter().filter(|e| e.pid == pid && e.pcr.is_some()) {
        let v = e.pcr.unwrap();
        if let Some(last) = last
            && !e.di
        {
            let d = pcr_delta(last, v);
            assert!(
                d > 0 && d <= PCR_NONE / 2,
                "backward PCR step {last} -> {v} at output packet {}",
                e.index
            );
        }
        last = Some(v);
    }
}

/// Stream-time spacing between consecutive same-era PCRs on `pid` is
/// within `max` ticks, checked over output packets `from ..`.
fn assert_spacing(events: &[Event], pid: u16, max: u64, from: usize) {
    let mut last: Option<u64> = None;
    for e in events
        .iter()
        .filter(|e| e.index >= from && e.pid == pid && e.pcr.is_some())
    {
        let v = e.pcr.unwrap();
        if !e.di
            && let Some(last) = last
        {
            let d = pcr_delta(last, v);
            assert!(
                d <= max,
                "PCR gap {d} ticks over {max} at output packet {}",
                e.index
            );
        }
        last = Some(v);
    }
}

/// Payload packets on every PID keep an unbroken CC sequence; AF-only
/// packets repeat the last payload CC on their PID.
fn assert_cc_integrity(events: &[Event]) {
    let mut cc: HashMap<u16, u8> = HashMap::new();
    for e in events {
        if e.pid == 0x1FFF {
            continue;
        }
        if e.has_payload {
            if let Some(prev) = cc.get(&e.pid) {
                assert_eq!(
                    e.cc,
                    (prev + 1) & 0x0F,
                    "CC break on pid {} at output packet {}",
                    e.pid,
                    e.index
                );
            }
            cc.insert(e.pid, e.cc);
        } else if let Some(prev) = cc.get(&e.pid) {
            assert_eq!(
                e.cc, *prev,
                "AF-only packet does not repeat CC on pid {} at output packet {}",
                e.pid, e.index
            );
        }
    }
}

/// Every PCR on `carrier` tracks the most recent decode timestamp seen on
/// `timing_pid` minus `lead`, within `tolerance` ticks.
fn assert_lead(events: &[Event], carrier: u16, timing_pid: u16, lead: u64, tolerance: u64) {
    let mut last_dts = None;
    let mut checked = 0;
    for e in events {
        if e.pid == timing_pid && e.dts.is_some() {
            last_dts = e.dts;
        }
        if e.pid == carrier
            && !e.di
            && let (Some(pcr), Some(dts)) = (e.pcr, last_dts)
        {
            let expect = expected_pcr(dts, lead);
            let dist = pcr_dist(expect, pcr);
            assert!(
                dist <= tolerance,
                "lead off by {dist} ticks at output packet {}",
                e.index
            );
            checked += 1;
        }
    }
    assert!(checked > 0, "no PCR was checked for lead");
}

/// Output equals the input with zero or more AF-only PCR packets added:
/// removing packets that are not byte-identical to the next expected input
/// packet must leave exactly the input. Valid only when the stage does not
/// modify packets (no restamp, no PMT patch).
fn assert_passthrough_plus_injections(out: &[u8], input: &[[u8; PACKET_SIZE]]) {
    let mut next = 0;
    for chunk in out.chunks_exact(PACKET_SIZE) {
        if next < input.len() && chunk == input[next] {
            next += 1;
        } else {
            let arr: &[u8; PACKET_SIZE] = chunk.try_into().unwrap();
            let ts = TsPacketRef::from(arr);
            assert!(
                ts.adaptation_field().and_then(|af| af.pcr()).is_some()
                    && ts.payload().is_none(),
                "unexpected non-injected packet in output"
            );
        }
    }
    assert_eq!(next, input.len(), "input packets lost or reordered");
}

fn flat(input: &[[u8; PACKET_SIZE]]) -> Vec<u8> {
    let mut v = Vec::with_capacity(input.len() * PACKET_SIZE);
    for p in input {
        v.extend_from_slice(p);
    }
    v
}

// Scenario 1: no PCR at all, PMT names the video PID as PCR_PID.
#[test]
fn auto_arms_full_without_pcr() {
    let mut f = Fixture::new(VIDEO_PID);
    for i in 0 .. 100 {
        f.av_frame(i, 90_000);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert_eq!(st.carrier_pid, Some(VIDEO_PID));
    assert_eq!(st.timing_pid, Some(VIDEO_PID));
    assert!(!st.pmt_patched);
    assert!(st.injected > 0);
    assert_eq!(st.restamped, 0);
    assert_eq!(st.discontinuities, 0);

    let pcrs = pcr_events(&ev, VIDEO_PID);
    assert!(!pcrs.is_empty());
    assert!(ev.iter().all(|e| e.pcr.is_none() || e.pid == VIDEO_PID));

    for e in &pcrs {
        assert!(!e.has_payload, "injected packet must be AF-only");
        assert_eq!(e.af_len, Some(183));
        assert_eq!(e.tsc, 0);
        assert!(!e.di);
    }

    // Auto probation: no PCR until ~1.5 s of continuous PES advance
    let first = pcrs[0].index;
    let dts_before = ev[.. first].iter().rev().find_map(|e| e.dts).unwrap();
    assert!(
        dts_before - 90_000 >= 135_000 - 3600,
        "armed before the 1.5 s probation: {dts_before}"
    );

    assert_monotone(&ev, VIDEO_PID);
    assert_spacing(&ev, VIDEO_PID, 40 * TICKS_MS, 0);
    assert_cc_integrity(&ev);
    assert_lead(&ev, VIDEO_PID, VIDEO_PID, LEAD_VIDEO, 150 * TICKS_MS);
    assert_passthrough_plus_injections(&out, &f.packets);
}

// Scenario 2: broken PCR (stuck, then garbage) leads to Full via the probe
// and the existing PCR fields are restamped onto the synthetic timeline.
fn run_broken_pcr(mut value: impl FnMut(u64) -> u64) {
    let mut f = Fixture::new(VIDEO_PID);
    for i in 0 .. 125 {
        f.av_frame(i, 90_000);
        if i % 2 == 0 {
            let v = value(i);
            f.pcr_packet(VIDEO_PID, v, false);
        }
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert!(st.restamped > 0, "existing PCR fields were not restamped");
    assert!(st.injected > 0);

    // After arming every PCR follows the synthetic timeline
    let pcrs = pcr_events(&ev, VIDEO_PID);
    let tail = &pcrs[pcrs.len() - 20 ..];
    let mut last_dts = None;
    for e in &ev[tail[0].index ..] {
        if e.dts.is_some() && e.pid == VIDEO_PID {
            last_dts = e.dts;
        }
        if let (Some(pcr), Some(dts)) = (e.pcr, last_dts) {
            let dist = pcr_dist(expected_pcr(dts, LEAD_VIDEO), pcr);
            assert!(
                dist <= 150 * TICKS_MS,
                "PCR not on the synthetic timeline at {}",
                e.index
            );
        }
    }

    assert_monotone(&ev[tail[0].index ..], VIDEO_PID);
    assert_spacing(&ev, VIDEO_PID, 40 * TICKS_MS, tail[0].index);
    assert_cc_integrity(&ev);
}

#[test]
fn broken_pcr_stuck_restamped() {
    run_broken_pcr(|_| 500_000);
}

#[test]
fn broken_pcr_garbage_restamped() {
    let mut seed = 7u64;
    run_broken_pcr(move |_| {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        seed % PCR_NONE
    });
}

// Scenario 3: sparse valid PCR arms TopUp; real PCRs pass byte-identical,
// injected values stay strictly between neighbouring reals; dense input
// recovers to Passive.
#[test]
fn sparse_pcr_topup_and_recovery() {
    let mut f = Fixture::new(VIDEO_PID);
    let mut reals = Vec::new();
    let mut next_pcr_ms = 0u64;
    for i in 0 .. 250 {
        f.av_frame(i, 90_000);
        let t_ms = i * 40;
        let dense = t_ms >= 6_000;
        if dense || t_ms >= next_pcr_ms {
            let dts = 90_000 + i * 3600;
            let v = expected_pcr(dts, 300 * TICKS_MS);
            f.pcr_packet(VIDEO_PID, v, false);
            reals.push(v);
            next_pcr_ms = t_ms + 500;
        }
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Passive, "did not recover to Passive");
    assert!(st.injected > 0, "TopUp never injected");
    assert_eq!(st.restamped, 0);
    assert_eq!(st.discontinuities, 0);

    // Every real PCR passes byte-identical (by value, in order), injected
    // values are strictly between neighbouring reals: the whole output
    // sequence is monotone
    let pcrs = pcr_events(&ev, VIDEO_PID);
    let mut real_iter = reals.iter();
    let mut matched = 0;
    for e in &pcrs {
        if real_iter.as_slice().first() == Some(&e.pcr.unwrap()) {
            real_iter.next();
            matched += 1;
        }
    }
    assert_eq!(matched, reals.len(), "a real PCR was lost or modified");

    assert_monotone(&ev, VIDEO_PID);
    assert_cc_integrity(&ev);

    // TopUp keeps the wire cadence under 40 ms while sparse; check spacing
    // between the 3 s and 5 s marks (fully within the TopUp region), and no
    // injections in the dense tail
    let in_topup: Vec<Event> = pcrs
        .iter()
        .filter(|e| {
            let v = e.pcr.unwrap();
            let lo = expected_pcr(90_000 + 75 * 3600, 300 * TICKS_MS);
            let hi = expected_pcr(90_000 + 125 * 3600, 300 * TICKS_MS);
            pcr_delta(lo, v) <= pcr_delta(lo, hi)
        })
        .copied()
        .collect();
    assert!(in_topup.len() > 10);
    assert_spacing(&in_topup, VIDEO_PID, 40 * TICKS_MS, 0);

    // Dense tail: PCRs are exactly the input reals
    let dense_reals: Vec<u64> = reals[reals.len() - 20 ..].to_vec();
    let tail: Vec<u64> = pcrs[pcrs.len() - 20 ..]
        .iter()
        .map(|e| e.pcr.unwrap())
        .collect();
    assert_eq!(tail, dense_reals, "injection continued after recovery");
}

// Scenario 5: healthy dense PCR keeps the stage Passive and transparent.
#[test]
fn healthy_pcr_stays_passive() {
    let mut f = Fixture::new(VIDEO_PID);
    for i in 0 .. 100 {
        f.av_frame(i, 90_000);
        let dts = 90_000 + i * 3600;
        f.pcr_packet(VIDEO_PID, expected_pcr(dts, 300 * TICKS_MS), false);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Passive);
    assert_eq!(st.injected, 0);
    assert_eq!(st.restamped, 0);
    assert_eq!(out, flat(&f.packets), "output must be byte-identical");
}

// Scenario 6: PTS/DTS wrap across 2^33 mid-stream, with one anchor landing
// exactly on the wrap point; the emitted PCR wraps mod PCR_NONE with no DI.
#[test]
fn pts_wrap_is_not_a_splice() {
    let start = MASK33 + 1 - 60 * 3600; // frame 60 lands exactly on 2^33
    let mut f = Fixture::new(VIDEO_PID);
    for i in 0 .. 120 {
        f.av_frame(i, start);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert_eq!(st.discontinuities, 0, "wrap must not emit a DI");

    let pcrs = pcr_events(&ev, VIDEO_PID);
    assert!(!pcrs.is_empty());
    assert!(pcrs.iter().all(|e| !e.di));
    assert!(pcrs.iter().all(|e| e.pcr.unwrap() < PCR_NONE));

    // Values on both sides of the PCR wrap, ordered wrap-aware
    assert!(pcrs.iter().any(|e| e.pcr.unwrap() > PCR_NONE - PCR_NONE / 8));
    assert!(pcrs.iter().any(|e| e.pcr.unwrap() < PCR_NONE / 8));
    assert_monotone(&ev, VIDEO_PID);
    assert_spacing(&ev, VIDEO_PID, 40 * TICKS_MS, 0);
    assert_lead(&ev, VIDEO_PID, VIDEO_PID, LEAD_VIDEO, 150 * TICKS_MS);
}

// Scenario 7: a splice landing near the wrap point is one splice (single DI),
// not a false wrap splice or a double DI.
#[test]
fn wrap_plus_splice_is_single_di() {
    let start = MASK33 + 1 - 40 * 3600;
    let mut f = Fixture::new(VIDEO_PID);
    // 1.8 s of normal advance to arm, ending 0.4 s before the wrap point
    for i in 0 .. 45 {
        f.av_frame(i, start - 45 * 3600);
    }
    // splice jumping across the wrap: forward 2 s
    for i in 0 .. 45 {
        f.av_frame(i, start - 45 * 3600 + 44 * 3600 + 180_000);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert_eq!(st.discontinuities, 1, "expected exactly one DI");

    let pcrs = pcr_events(&ev, VIDEO_PID);
    assert_eq!(pcrs.iter().filter(|e| e.di).count(), 1);
    assert_monotone(&ev, VIDEO_PID);

    // The timeline follows the jump
    assert_lead(&ev, VIDEO_PID, VIDEO_PID, LEAD_VIDEO, 150 * TICKS_MS);
}

// Scenario 8: a splice of 1 s or more emits the DI-flagged PCR packet before
// the triggering timing packet, and the lead is restored.
#[test]
fn splice_di_precedes_trigger() {
    let mut f = Fixture::new(VIDEO_PID);
    for i in 0 .. 60 {
        f.av_frame(i, 90_000);
    }
    let jump = 90_000 + 59 * 3600 + 450_000; // +5 s
    for i in 0 .. 30 {
        f.av_frame(i, jump);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    assert_eq!(s.status().discontinuities, 1);

    let di = ev
        .iter()
        .find(|e| e.di && e.pcr.is_some())
        .expect("DI packet");
    assert_eq!(di.pid, VIDEO_PID);
    assert!(!di.has_payload);

    // The next output packet is the timing packet that triggered the splice
    let trigger = &ev[di.index + 1];
    assert_eq!(trigger.pid, VIDEO_PID);
    assert_eq!(trigger.dts, Some(jump & MASK33));

    // The lead is fully restored on the DI packet
    assert_eq!(di.pcr.unwrap(), expected_pcr(jump, LEAD_VIDEO));

    assert_monotone(&ev, VIDEO_PID);
    assert_cc_integrity(&ev);
}

/// Reassembles PSI sections from the PMT PID packets of a byte stream.
fn pmt_versions(out: &[u8]) -> Vec<(u8, u16, usize)> {
    let mut psi = Psi::default();
    let mut result = Vec::new();
    for chunk in out.chunks_exact(PACKET_SIZE) {
        let arr: &[u8; PACKET_SIZE] = chunk.try_into().unwrap();
        let ts = TsPacketRef::from(arr);
        if ts.pid() != PMT_PID {
            continue;
        }
        psi.assemble(&ts);
        if let Some(section) = psi.sections().first() {
            let pmt = PmtSectionRef::try_from(section).expect("CRC-valid PMT in output");
            result.push((pmt.version(), pmt.pcr_pid(), section.len()));
        }
    }
    result
}

// Scenario 4: PMT PCR_PID=0x1FFF arms Full without probation; the first PMT
// repetition passes verbatim, later ones are patched in place; an upstream
// version bump re-learns and patches with the new version + 1.
#[test]
fn pmt_pcr_pid_none_is_patched() {
    let mut f = Fixture::new(0x1FFF);
    f.set_pmt(0x1FFF, 1, 220); // two-packet PMT section
    for i in 0 .. 60 {
        f.av_frame(i, 90_000);
    }
    // upstream version bump with changed content
    f.set_pmt(0x1FFF, 5, 240);
    for i in 60 .. 120 {
        f.av_frame(i, 90_000);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert!(st.pmt_patched);
    assert_eq!(st.carrier_pid, Some(VIDEO_PID));

    // Structural arming skips the 1.5 s probation: first PCR well before it
    let first = ev.iter().find(|e| e.pcr.is_some()).expect("PCR emitted");
    let dts_before = ev[.. first.index].iter().rev().find_map(|e| e.dts).unwrap();
    assert!(
        dts_before - 90_000 < 45_000,
        "structural arming did not skip probation: {dts_before}"
    );

    let versions = pmt_versions(&out);
    let input_versions = pmt_versions(&flat(&f.packets));
    assert_eq!(versions.len(), input_versions.len(), "PMT section count changed");

    // First sighting verbatim, repetitions patched with version + 1; the
    // bumped upstream section repeats the pattern
    assert_eq!(versions[0], (1, 0x1FFF, input_versions[0].2));
    assert!(versions[1 .. 5].iter().all(|v| *v == (2, VIDEO_PID, input_versions[0].2)));
    let bump = versions
        .iter()
        .position(|v| v.0 == 5)
        .expect("upstream bump passes verbatim once");
    assert_eq!(versions[bump], (5, 0x1FFF, input_versions[bump].2));
    assert!(versions[bump + 1 ..].iter().all(|v| *v == (6, VIDEO_PID, input_versions[bump].2)));
    assert!(versions.len() > bump + 2);

    // Packet count, CC and PSI timing untouched
    assert_eq!(
        ev.iter().filter(|e| e.pid == PMT_PID).count(),
        f.packets
            .iter()
            .filter(|p| (u16::from(p[1] & 0x1F) << 8) | u16::from(p[2]) == PMT_PID)
            .count()
    );
    assert_cc_integrity(&ev);
    assert_monotone(&ev, VIDEO_PID);
    assert_spacing(&ev, VIDEO_PID, 40 * TICKS_MS, first.index);
}

// Scenario 9: PTS-only B-frame pattern; decode-order backward steps are
// reorders, not splices - no DI storm over many GOPs.
#[test]
fn bframe_reorder_no_di_storm() {
    let mut f = Fixture::new(VIDEO_PID);
    // decode order per GOP of 3: P(n+3d) B(n+1d) B(n+2d)
    let d = 3600u64;
    let mut n = 0u64;
    for gop in 0 .. 100u64 {
        if gop % 4 == 0 {
            f.psi();
        }
        f.video_frame(90_000 + n + 3 * d, None, 700);
        f.video_frame(90_000 + n + d, None, 700);
        f.video_frame(90_000 + n + 2 * d, None, 700);
        n += 3 * d;
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert_eq!(st.discontinuities, 0, "DI storm on B-frame reorders");
    assert!(st.injected > 0);

    assert_monotone(&ev, VIDEO_PID);
    // rate stays sane: cadence bound holds after arming
    let first = ev.iter().find(|e| e.pcr.is_some()).unwrap().index;
    assert_spacing(&ev, VIDEO_PID, 40 * TICKS_MS, first);
}

// Scenario 10: scrambled video with clear audio; the audio PID drives the
// clock and the default lead is 150 ms.
#[test]
fn scrambled_video_uses_audio_timing() {
    let mut f = Fixture::new(AUDIO_PID);
    for i in 0 .. 150u64 {
        let t = 90_000 + i * 3600;
        if i % 12 == 0 {
            f.psi();
        }
        f.scrambled_video_frame(t + 7200, 700);
        f.audio_frame(t, 200);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert_eq!(st.timing_pid, Some(AUDIO_PID));
    assert_eq!(st.carrier_pid, Some(AUDIO_PID));

    assert_monotone(&ev, AUDIO_PID);
    assert_lead(&ev, AUDIO_PID, AUDIO_PID, LEAD_OTHER, 150 * TICKS_MS);
    assert_cc_integrity(&ev);
}

// Scenario 11: audio-only stream at ~192 kbit/s; the cadence bound holds in
// stream time.
#[test]
fn audio_only_low_bitrate_cadence() {
    let mut f = Fixture::new(AUDIO_PID);
    for i in 0 .. 250u64 {
        if i % 20 == 0 {
            f.psi();
        }
        // 24 ms per frame, 576 bytes = 192 kbit/s
        f.audio_frame(90_000 + i * 2160, 576);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert_eq!(st.timing_pid, Some(AUDIO_PID));

    let first = ev.iter().find(|e| e.pcr.is_some()).unwrap().index;
    assert_monotone(&ev, AUDIO_PID);
    assert_spacing(&ev, AUDIO_PID, 40 * TICKS_MS, first);
    assert_lead(&ev, AUDIO_PID, AUDIO_PID, LEAD_OTHER, 150 * TICKS_MS);
    assert_cc_integrity(&ev);
}

// Scenario 12: segment_boundary + moderate PTS jump is an immediate splice;
// segment_boundary + tiny jump is not.
#[test]
fn segment_boundary_moderate_jump_is_splice() {
    let mut f = Fixture::new(VIDEO_PID);
    for i in 0 .. 60 {
        f.av_frame(i, 90_000);
    }
    let mut s = PcrSynth::new(auto());
    let mut out = run(&mut s, &f.packets);
    assert_eq!(s.status().discontinuities, 0);

    s.segment_boundary();

    // 300 ms jump right after the boundary
    let mut g = Fixture::new(VIDEO_PID);
    g.video = PesPacketizer::new(VIDEO_PID);
    let jump = 90_000 + 59 * 3600 + 27_000;
    for i in 0 .. 30 {
        g.video_frame(jump + i * 3600 + 7200, Some(jump + i * 3600), 700);
    }
    for p in &g.packets {
        s.process(p, &mut out);
    }

    assert_eq!(s.status().discontinuities, 1, "expected an immediate splice DI");
    assert_monotone(&events(&out), VIDEO_PID);
}

#[test]
fn segment_boundary_tiny_jump_is_not_a_splice() {
    let mut f = Fixture::new(VIDEO_PID);
    for i in 0 .. 60 {
        f.av_frame(i, 90_000);
    }
    let mut s = PcrSynth::new(auto());
    let mut out = run(&mut s, &f.packets);

    s.segment_boundary();

    // 120 ms jump (under the 200 ms threshold) right after the boundary
    let mut g = Fixture::new(VIDEO_PID);
    let jump = 90_000 + 59 * 3600 + 10_800;
    for i in 0 .. 30 {
        g.video_frame(jump + i * 3600 + 7200, Some(jump + i * 3600), 700);
    }
    for p in &g.packets {
        s.process(p, &mut out);
    }

    assert_eq!(s.status().discontinuities, 0, "tiny jump must not splice");
    assert_monotone(&events(&out), VIDEO_PID);
}

// Scenario 13: more than one program disables the stage.
#[test]
fn multiprogram_is_transparent() {
    let mut pat = PsiPacketizer::new(0);
    pat.set_sections(PatBuilder::build(PatConfig {
        transport_stream_id: 1,
        version: 0,
        programs: vec![
            PatProgram {
                program_number: 1,
                pid: PMT_PID,
            },
            PatProgram {
                program_number: 2,
                pid: PMT_PID + 1,
            },
        ],
    }));

    let mut f = Fixture::new(VIDEO_PID);
    f.pat = pat;
    for i in 0 .. 100 {
        f.av_frame(i, 90_000);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::MultiProgram);
    assert_eq!(st.injected, 0);
    assert_eq!(out, flat(&f.packets));
}

// Scenario 14: everything scrambled and no PCR: transparent, NoTiming.
#[test]
fn scrambled_no_pcr_is_no_timing() {
    let mut f = Fixture::new(VIDEO_PID);
    let mut i = 0u64;
    while f.packets.len() * PACKET_SIZE < 2 * 1024 * 1024 + 4 * PACKET_SIZE {
        if i.is_multiple_of(12) {
            f.psi();
        }
        f.scrambled_video_frame(90_000 + i * 3600, 700);
        i += 1;
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::NoTiming);
    assert_eq!(st.injected, 0);
    assert_eq!(out, flat(&f.packets));
}

// Scenario 15: Force mode synthesizes from the first anchor + rate, without
// the 1.5 s probation.
#[test]
fn force_mode_skips_probation() {
    let mut f = Fixture::new(VIDEO_PID);
    for i in 0 .. 30 {
        f.av_frame(i, 90_000);
    }

    let mut s = PcrSynth::new(PcrSynthConfig {
        mode: PcrSynthMode::Force,
        pcr_delay_ms: None,
    });
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert!(st.injected > 0);

    let first = ev.iter().find(|e| e.pcr.is_some()).expect("PCR emitted");
    let dts_before = ev[.. first.index].iter().rev().find_map(|e| e.dts).unwrap();
    assert!(
        dts_before - 90_000 < 45_000,
        "Force mode waited too long: {dts_before}"
    );
    assert_monotone(&ev, VIDEO_PID);
    assert_lead(&ev, VIDEO_PID, VIDEO_PID, LEAD_VIDEO, 150 * TICKS_MS);
}

// Scenario 16: a byte-rate over-estimate (idle stuffing flood between
// anchors) exhausts the 100 ms clamp budget and forces a DI re-anchor that
// restores the full lead. Run once mid-range and once with the clamp
// crossing the PCR_NONE wrap boundary.
fn run_clamp_budget(dts_start: u64) {
    let mut f = Fixture::new(VIDEO_PID);
    for i in 0 .. 46 {
        f.av_frame(i, dts_start);
    }
    for _ in 0 .. 300 {
        f.null_packet();
    }
    for i in 46 .. 76 {
        f.av_frame(i, dts_start);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert_eq!(st.discontinuities, 1, "clamp budget DI expected exactly once");

    assert!(ev.iter().all(|e| e.pcr.is_none_or(|v| v < PCR_NONE)));
    assert_monotone(&ev, VIDEO_PID);

    // The lead is restored after the DI re-anchor
    let di = ev.iter().find(|e| e.di).expect("DI packet").index;
    assert_lead(&ev[di ..], VIDEO_PID, VIDEO_PID, LEAD_VIDEO, 150 * TICKS_MS);
}

#[test]
fn clamp_budget_forces_di() {
    run_clamp_budget(90_000);
}

#[test]
fn clamp_budget_di_across_pcr_wrap() {
    run_clamp_budget(MASK33 + 1 - 230_000);
}

// Scenario 17: random and malformed packets never panic and pass through.
#[test]
fn fuzz_random_packets_never_panic() {
    let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
    let mut next = move || {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        seed
    };

    for mode in [PcrSynthMode::Auto, PcrSynthMode::Force] {
        let mut s = PcrSynth::new(PcrSynthConfig {
            mode,
            pcr_delay_ms: Some(100),
        });
        let mut out = Vec::new();
        for i in 0 .. 20_000usize {
            let mut p = [0u8; PACKET_SIZE];
            for chunk in p.chunks_mut(8) {
                let bytes = next().to_le_bytes();
                let n = chunk.len();
                chunk.copy_from_slice(&bytes[.. n]);
            }
            if i % 2 == 0 {
                p[0] = 0x47;
            }
            if i % 16 == 0 {
                // poke the PAT path with garbage payloads
                p[0] = 0x47;
                p[1] &= 0xE0;
                p[2] = 0;
            }
            if i % 1000 == 0 {
                s.segment_boundary();
            }
            s.process(&p, &mut out);
        }
        assert!(out.len().is_multiple_of(PACKET_SIZE));
        assert!(out.len() / PACKET_SIZE >= 20_000, "input packets were lost");
        let _ = s.status();
    }
}

// Scenario 17b: garbage after Full is armed exercises the emission paths.
#[test]
fn fuzz_after_full_never_panics() {
    let mut f = Fixture::new(0x1FFF);
    for i in 0 .. 60 {
        f.av_frame(i, 90_000);
    }
    let mut s = PcrSynth::new(auto());
    let mut out = run(&mut s, &f.packets);
    assert_eq!(s.status().phase, PcrSynthPhase::Full);

    let mut seed: u64 = 0xDEAD_BEEF_CAFE_F00D;
    for i in 0 .. 5_000usize {
        let mut p = [0u8; PACKET_SIZE];
        for chunk in p.chunks_mut(8) {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let bytes = seed.to_le_bytes();
            let n = chunk.len();
            chunk.copy_from_slice(&bytes[.. n]);
        }
        p[0] = 0x47;
        if i % 8 == 0 {
            // garbage on the tracked PMT PID hits the streaming patch path
            p[1] = (p[1] & 0xE0) | ((PMT_PID >> 8) as u8);
            p[2] = PMT_PID as u8;
        }
        s.process(&p, &mut out);
    }
    assert!(out.len().is_multiple_of(PACKET_SIZE));
    let _ = s.status();
}

// Scenario 18: a fresh instance reports Probing with zeroed counters; the
// per-scenario counter movements are asserted in scenarios 1-4.
#[test]
fn initial_status_snapshot() {
    let s = PcrSynth::new(auto());
    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Probing);
    assert_eq!(st.carrier_pid, None);
    assert_eq!(st.timing_pid, None);
    assert!(!st.pmt_patched);
    assert_eq!(st.injected, 0);
    assert_eq!(st.restamped, 0);
    assert_eq!(st.discontinuities, 0);
}

// Probe: the timing PID goes silent; the clock keeps extrapolating over the
// byte position and the cadence bound holds with no false DI.
#[test]
fn timing_pid_silence_keeps_cadence() {
    let mut f = Fixture::new(VIDEO_PID);
    for i in 0 .. 60 {
        f.av_frame(i, 90_000);
    }
    let silent_from = f.packets.len();
    for _ in 0 .. 800 {
        f.null_packet();
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    assert_eq!(s.status().discontinuities, 0);

    // Injections continue through the silent span
    let silent_pcrs = ev
        .iter()
        .filter(|e| e.index >= silent_from && e.pcr.is_some())
        .count();
    assert!(silent_pcrs > 20, "injection stopped during timing silence");

    assert_monotone(&ev, VIDEO_PID);
    let first = ev.iter().find(|e| e.pcr.is_some()).unwrap().index;
    assert_spacing(&ev, VIDEO_PID, 40 * TICKS_MS, first);
}

// Probe: the PMT PCR_PID names a PID that never exists in the stream; it is
// still the carrier and injections create it (CC 0 throughout).
#[test]
fn ghost_pcr_pid_carries_injections() {
    let ghost = 900u16;
    let mut f = Fixture::new(ghost);
    for i in 0 .. 100 {
        f.av_frame(i, 90_000);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert_eq!(st.carrier_pid, Some(ghost));
    assert!(st.injected > 0);

    let pcrs = pcr_events(&ev, ghost);
    assert!(!pcrs.is_empty());
    assert!(ev.iter().all(|e| e.pcr.is_none() || e.pid == ghost));
    assert!(pcrs.iter().all(|e| e.cc == 0 && !e.has_payload));

    assert_monotone(&ev, ghost);
    assert_spacing(&ev, ghost, 40 * TICKS_MS, 0);
    assert_cc_integrity(&ev);
}

// Probe: PMT arrives long before any PES; the stage stays transparent until
// the clock exists, then synthesizes.
#[test]
fn pmt_before_any_pes() {
    let mut f = Fixture::new(VIDEO_PID);
    f.psi();
    for _ in 0 .. 500 {
        f.null_packet();
    }
    let quiet_end = f.packets.len();
    for i in 0 .. 60 {
        f.av_frame(i, 90_000);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    assert_eq!(s.status().phase, PcrSynthPhase::Full);
    let first = ev.iter().find(|e| e.pcr.is_some()).expect("PCR emitted");
    assert!(
        first.index > quiet_end,
        "injected before any timing information existed"
    );
    assert_monotone(&ev, VIDEO_PID);
}

// Probe: zero-length adaptation fields on the timing packets, one packet per
// 200 ms (extreme low bitrate: a single packet spans several cadence
// targets). One injection precedes each packet; the clock stays monotone.
#[test]
fn zero_length_af_and_extreme_low_bitrate() {
    let pid = 999u16;

    let mut pat = PsiPacketizer::new(0);
    pat.set_sections(PatBuilder::build(PatConfig {
        transport_stream_id: 1,
        version: 0,
        programs: vec![PatProgram {
            program_number: 1,
            pid: PMT_PID,
        }],
    }));
    let mut pmt = PsiPacketizer::new(PMT_PID);
    pmt.set_sections(PmtBuilder::build(PmtConfig {
        program_number: 1,
        pcr_pid: pid,
        version: 0,
        program_descriptors: Vec::new(),
        streams: vec![PmtStream {
            stream_type: 0x1B,
            elementary_pid: pid,
            stream_descriptors: Vec::new(),
        }],
    }));

    let mut packets: Vec<[u8; PACKET_SIZE]> = Vec::new();
    let mut packet = [0u8; PACKET_SIZE];
    while pat.next(&mut packet) {
        packets.push(packet);
    }
    while pmt.next(&mut packet) {
        packets.push(packet);
    }

    for i in 0 .. 40u64 {
        let mut p = [0xFFu8; PACKET_SIZE];
        {
            let mut m = TsPacketMut::from(&mut p);
            m.init(pid, (i & 0x0F) as u8);
            m.set_payload();
            m.set_pusi();
        }
        p[3] |= 0x20; // adaptation field flag
        p[4] = 0; // zero-length adaptation field
        let header = PesHeader::new(STREAM_ID_VIDEO)
            .with_pts_dts(PtsDts::new(90_000 + i * 18_000)); // 200 ms steps
        let mut tmp = [0u8; 32];
        let n = header.write(&mut tmp);
        p[5 .. 5 + n].copy_from_slice(&tmp[.. n]);
        packets.push(p);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert_eq!(st.timing_pid, Some(pid));
    assert!(st.injected > 0);
    assert_eq!(st.discontinuities, 0);

    assert_monotone(&ev, pid);
    // One injection per input packet: consecutive values step by about the
    // packet span (200 ms), never more than 250 ms
    assert_spacing(&ev, pid, 250 * TICKS_MS, 0);
    assert_cc_integrity(&ev);
}

/// PCR values across ALL PIDs are strictly monotone wrap-aware within eras.
fn assert_monotone_all(events: &[Event]) {
    let mut last: Option<u64> = None;
    for e in events.iter().filter(|e| e.pcr.is_some()) {
        let v = e.pcr.unwrap();
        if let Some(last) = last
            && !e.di
        {
            let d = pcr_delta(last, v);
            assert!(
                d > 0 && d <= PCR_NONE / 2,
                "global backward PCR step {last} -> {v} at output packet {}",
                e.index
            );
        }
        last = Some(v);
    }
}

// Probe: broken PCR on a foreign PID (not the carrier). Full mode restamps
// it onto the synthetic timeline; the whole output PCR sequence across both
// PIDs is monotone.
#[test]
fn foreign_pid_pcr_restamped_globally_monotone() {
    let mut f = Fixture::new(VIDEO_PID);
    for i in 0 .. 125 {
        f.av_frame(i, 90_000);
        if i % 2 == 0 {
            f.pcr_packet(AUDIO_PID, 777_777, false); // stuck, on the audio PID
        }
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert_eq!(st.carrier_pid, Some(VIDEO_PID));
    assert!(st.restamped > 0);
    assert!(st.injected > 0);

    // Before arming the stuck values pass through; the first synthetic
    // emission is an injection on the carrier. From that point on every
    // stuck audio PCR is restamped and the sequence is globally monotone
    // across the audio restamps and the video injections
    let armed_at = ev
        .iter()
        .find(|e| e.pcr.is_some() && e.pid == VIDEO_PID)
        .unwrap()
        .index;
    let tail: Vec<Event> = ev[armed_at ..].to_vec();
    assert!(
        tail.iter().all(|e| e.pcr != Some(777_777)),
        "a stuck foreign PCR survived the restamp"
    );
    assert_monotone_all(&tail);
    assert_cc_integrity(&ev);
}

// Probe: TopUp with the sparse real PCRs crossing the PCR_NONE wrap.
#[test]
fn topup_across_pcr_wrap() {
    let start = MASK33 + 1 - 90_000; // wrap 1 s into the stream
    let mut f = Fixture::new(VIDEO_PID);
    let mut reals = Vec::new();
    let mut next_pcr_ms = 0u64;
    for i in 0 .. 150 {
        f.av_frame(i, start);
        let t_ms = i * 40;
        if t_ms >= next_pcr_ms {
            let v = expected_pcr(start + i * 3600, 100 * TICKS_MS);
            f.pcr_packet(VIDEO_PID, v, false);
            reals.push(v);
            next_pcr_ms = t_ms + 500;
        }
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::TopUp);
    assert!(st.injected > 0);
    assert_eq!(st.discontinuities, 0);

    // Reals pass unmodified and in order; the whole sequence is monotone
    // across the wrap
    let pcrs = pcr_events(&ev, VIDEO_PID);
    let values: Vec<u64> = pcrs.iter().map(|e| e.pcr.unwrap()).collect();
    let mut real_iter = reals.iter().peekable();
    for v in &values {
        if real_iter.peek() == Some(&v) {
            real_iter.next();
        }
    }
    assert!(real_iter.peek().is_none(), "a real PCR was lost across the wrap");
    assert!(values.iter().any(|v| *v > PCR_NONE - PCR_NONE / 8));
    assert!(values.iter().any(|v| *v < PCR_NONE / 8));
    assert_monotone(&ev, VIDEO_PID);
}

// Probe: PAT flips single -> multi -> single program; the stage is
// transparent while MultiProgram and resumes afterwards.
#[test]
fn pat_multiprogram_flip_recovers() {
    let mut f = Fixture::new(VIDEO_PID);
    for i in 0 .. 60 {
        f.av_frame(i, 90_000);
    }

    let mut s = PcrSynth::new(auto());
    let mut out = run(&mut s, &f.packets);
    assert_eq!(s.status().phase, PcrSynthPhase::Full);

    // Multi-program PAT: transparent region
    f.pat.set_sections(PatBuilder::build(PatConfig {
        transport_stream_id: 1,
        version: 1,
        programs: vec![
            PatProgram {
                program_number: 1,
                pid: PMT_PID,
            },
            PatProgram {
                program_number: 2,
                pid: PMT_PID + 1,
            },
        ],
    }));
    let before_multi = f.packets.len();
    for i in 60 .. 90 {
        f.av_frame(i, 90_000);
    }
    let multi_region: Vec<[u8; PACKET_SIZE]> = f.packets[before_multi ..].to_vec();
    let out_len_before = out.len();
    for p in &multi_region {
        s.process(p, &mut out);
    }
    // skip the packets until the new PAT is seen, then fully transparent
    assert_eq!(s.status().phase, PcrSynthPhase::MultiProgram);
    let multi_out = &out[out_len_before ..];
    assert!(multi_out.len() <= multi_region.len() * PACKET_SIZE + 2 * PACKET_SIZE);

    // Back to a single program: synthesis resumes
    f.pat.set_sections(PatBuilder::build(PatConfig {
        transport_stream_id: 1,
        version: 2,
        programs: vec![PatProgram {
            program_number: 1,
            pid: PMT_PID,
        }],
    }));
    let before_single = f.packets.len();
    for i in 90 .. 150 {
        f.av_frame(i, 90_000);
    }
    let injected_before = s.status().injected;
    let out_len_single = out.len();
    for p in &f.packets[before_single ..] {
        s.process(p, &mut out);
    }
    assert_eq!(s.status().phase, PcrSynthPhase::Full);
    assert!(s.status().injected > injected_before, "did not resume injecting");
    assert_monotone(&events(&out[out_len_single ..]), VIDEO_PID);
}

/// Builds one PMT packet with an explicit pointer_field: `head` bytes of the
/// previous section (or filler) before the new section start.
fn pmt_packet_with_pointer(
    cc: u8,
    pointer: u8,
    head: &[u8],
    body: &[u8],
    pusi: bool,
) -> [u8; PACKET_SIZE] {
    let mut p = [0xFFu8; PACKET_SIZE];
    {
        let mut m = TsPacketMut::from(&mut p);
        m.init(PMT_PID, cc);
        m.set_payload();
        if pusi {
            m.set_pusi();
        }
    }
    let mut offset = 4;
    if pusi {
        p[offset] = pointer;
        offset += 1;
        p[offset .. offset + head.len()].copy_from_slice(head);
        offset += head.len();
    }
    p[offset .. offset + body.len()].copy_from_slice(body);
    p
}

// Probe: the streaming PMT patch across non-zero pointer_field packets:
// section tail and next section head share one packet.
#[test]
fn pmt_patch_handles_pointer_field() {
    let sections = pmt_sections(0x1FFF, 1, 204); // section of 230 bytes
    let section: Vec<u8> = sections[0].to_vec();
    assert_eq!(section.len(), 230);

    let mut f = Fixture::new(VIDEO_PID);
    // video timing first so that structural arming can happen right after
    // the PMT is learned
    f.pat.reset();
    let mut packet = [0u8; PACKET_SIZE];
    while f.pat.next(&mut packet) {
        f.push(packet);
    }
    for i in 0 .. 12 {
        f.video_frame(90_000 + i * 3600 + 7200, Some(90_000 + i * 3600), 700);
    }

    // Repetition pairs: pkt A = PUSI ptr 0 + section[0 .. 183],
    // pkt B = PUSI ptr 47 + section[183 ..] + next section[0 .. 136],
    // pkt C = continuation section[136 ..] + stuffing
    let mut cc = 0u8;
    let mut push_rep_chain = |f: &mut Fixture, reps: usize| {
        // first section head
        f.packets.push(pmt_packet_with_pointer(cc, 0, &[], &section[.. 183], true));
        cc = (cc + 1) & 0x0F;
        for _ in 0 .. reps {
            f.packets.push(pmt_packet_with_pointer(
                cc,
                47,
                &section[183 ..],
                &section[.. 136],
                true,
            ));
            cc = (cc + 1) & 0x0F;
        }
        // last tail as a plain continuation
        f.packets.push(pmt_packet_with_pointer(cc, 0, &[], &section[136 ..], false));
        cc = (cc + 1) & 0x0F;
    };

    push_rep_chain(&mut f, 1);
    for i in 12 .. 24 {
        f.video_frame(90_000 + i * 3600 + 7200, Some(90_000 + i * 3600), 700);
    }
    push_rep_chain(&mut f, 1);
    for i in 24 .. 36 {
        f.video_frame(90_000 + i * 3600 + 7200, Some(90_000 + i * 3600), 700);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert!(st.pmt_patched);

    let versions = pmt_versions(&out);
    assert_eq!(versions.len(), 4);
    // The very first section passes verbatim; every following repetition is
    // patched, including the ones flowing through pointer_field packets
    assert_eq!(versions[0], (1, 0x1FFF, 230));
    assert!(versions[1 ..].iter().all(|v| *v == (2, VIDEO_PID, 230)));
}

// Probe: a malformed pointer_field (larger than the payload) mid-flow passes
// verbatim without a panic and patching resumes on the next repetition.
#[test]
fn pmt_malformed_pointer_is_verbatim() {
    let mut f = Fixture::new(0x1FFF);
    for i in 0 .. 40 {
        f.av_frame(i, 90_000);
    }
    // malformed PMT packet: PUSI with pointer_field beyond the payload
    let mut bad = [0xFFu8; PACKET_SIZE];
    {
        let mut m = TsPacketMut::from(&mut bad);
        m.init(PMT_PID, 9);
        m.set_payload();
        m.set_pusi();
    }
    bad[4] = 200;
    f.packets.push(bad);
    for i in 40 .. 80 {
        f.av_frame(i, 90_000);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);

    assert_eq!(s.status().phase, PcrSynthPhase::Full);
    assert!(s.status().pmt_patched);

    // The malformed packet is passed through byte-identical
    assert!(
        out.chunks_exact(PACKET_SIZE).any(|c| c == bad),
        "malformed PMT packet was modified or lost"
    );

    // Patching still works on later repetitions
    let mut psi = Psi::default();
    let mut patched = 0;
    for chunk in out.chunks_exact(PACKET_SIZE) {
        let arr: &[u8; PACKET_SIZE] = chunk.try_into().unwrap();
        let ts = TsPacketRef::from(arr);
        if ts.pid() != PMT_PID {
            continue;
        }
        psi.assemble(&ts);
        if let Some(section) = psi.sections().first()
            && let Ok(pmt) = PmtSectionRef::try_from(section)
            && pmt.pcr_pid() == VIDEO_PID
        {
            patched += 1;
        }
    }
    assert!(patched > 2, "patching did not resume after the malformed packet");
}

// Probe: no PMT at all; after 2 MiB of stream the timing PID becomes the
// carrier and injection starts, with no PMT patch.
#[test]
fn no_pmt_falls_back_to_timing_carrier() {
    let mut packets: Vec<[u8; PACKET_SIZE]> = Vec::new();
    let mut video = PesPacketizer::new(VIDEO_PID);
    let mut i = 0u64;
    while packets.len() * PACKET_SIZE < 2 * 1024 * 1024 + 200 * PACKET_SIZE {
        let pts_dts = PtsDts::new((90_000 + i * 3600 + 7200) & MASK33)
            .with_dts((90_000 + i * 3600) & MASK33);
        let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(pts_dts);
        video.set_frame(EsFrame {
            header,
            payload: vec![0u8; 700],
            rai: false,
        });
        let mut packet = [0u8; PACKET_SIZE];
        while video.next(&mut packet) {
            packets.push(packet);
        }
        i += 1;
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert_eq!(st.carrier_pid, Some(VIDEO_PID));
    assert!(!st.pmt_patched);
    assert!(st.injected > 0);

    // Transparent until the PMT wait expired
    let first = ev.iter().find(|e| e.pcr.is_some()).unwrap().index;
    assert!(
        first * PACKET_SIZE >= 2 * 1024 * 1024 - PACKET_SIZE,
        "carrier elected before the PMT wait expired: packet {first}"
    );
    assert_monotone(&ev, VIDEO_PID);
    assert_spacing(&ev, VIDEO_PID, 40 * TICKS_MS, first);
}

// Probe: pcr_delay_ms override applies, and an extreme override does not
// break the wrap arithmetic.
#[test]
fn pcr_delay_override() {
    let mut f = Fixture::new(VIDEO_PID);
    for i in 0 .. 80 {
        f.av_frame(i, 90_000);
    }

    let mut s = PcrSynth::new(PcrSynthConfig {
        mode: PcrSynthMode::Auto,
        pcr_delay_ms: Some(200),
    });
    let out = run(&mut s, &f.packets);
    let ev = events(&out);
    assert_lead(&ev, VIDEO_PID, VIDEO_PID, 200 * TICKS_MS, 150 * TICKS_MS);

    // Extreme override: values stay valid and monotone
    let mut s = PcrSynth::new(PcrSynthConfig {
        mode: PcrSynthMode::Auto,
        pcr_delay_ms: Some(u64::MAX),
    });
    let out = run(&mut s, &f.packets);
    let ev = events(&out);
    assert!(ev.iter().all(|e| e.pcr.is_none_or(|v| v < PCR_NONE)));
    assert_monotone(&ev, VIDEO_PID);
}

// Probe: NoTiming is not terminal; a stream that becomes clear later still
// arms full synthesis.
#[test]
fn no_timing_recovers_when_stream_clears() {
    let mut f = Fixture::new(VIDEO_PID);
    let mut i = 0u64;
    while f.packets.len() * PACKET_SIZE < 2 * 1024 * 1024 + 4 * PACKET_SIZE {
        f.scrambled_video_frame(90_000 + i * 3600, 700);
        i += 1;
    }

    let mut s = PcrSynth::new(auto());
    let mut out = run(&mut s, &f.packets);
    assert_eq!(s.status().phase, PcrSynthPhase::NoTiming);

    let clear_from = f.packets.len();
    for j in 0 .. 60 {
        f.av_frame(i + j, 90_000 + (i + j) * 3600);
    }
    for p in &f.packets[clear_from ..] {
        s.process(p, &mut out);
    }

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert!(st.injected > 0);
}

// Probe: a backward decode-time jump of 1 s or more is a splice too; the
// timeline follows down with a single DI.
#[test]
fn backward_splice_follows_timeline() {
    let mut f = Fixture::new(VIDEO_PID);
    for i in 0 .. 60 {
        f.av_frame(i, 450_000);
    }
    let jump = 450_000 + 59 * 3600 - 270_000; // -3 s
    for i in 0 .. 30 {
        f.av_frame(i, jump);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    assert_eq!(s.status().discontinuities, 1);
    let di = ev.iter().find(|e| e.di && e.pcr.is_some()).expect("DI packet");
    assert_eq!(di.pcr.unwrap(), expected_pcr(jump, LEAD_VIDEO));
    assert_eq!(ev[di.index + 1].dts, Some(jump & MASK33));
    assert_monotone(&ev, VIDEO_PID);
    assert_lead(&ev[di.index ..], VIDEO_PID, VIDEO_PID, LEAD_VIDEO, 150 * TICKS_MS);
}

// Probe: two clear non-video PIDs with strictly alternating PES starts (a
// two-track audio mux, no clear video); the first PID commits on its second
// sighting and full synthesis arms.
#[test]
fn alternating_audio_pids_commit_timing() {
    let second_pid = AUDIO_PID + 1;
    let mut f = Fixture::new(0x1FFF);
    let mut second = PesPacketizer::new(second_pid);
    for i in 0 .. 250u64 {
        if i % 20 == 0 {
            f.psi();
        }
        // 24 ms cadence per track, PES starts strictly alternating
        f.audio_frame(90_000 + i * 2160, 576);
        let header = PesHeader::new(STREAM_ID_AUDIO)
            .with_pts_dts(PtsDts::new(90_000 + 1080 + i * 2160));
        second.set_frame(EsFrame {
            header,
            payload: vec![0u8; 576],
            rai: false,
        });
        let mut packet = [0u8; PACKET_SIZE];
        while second.next(&mut packet) {
            f.push(packet);
        }
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert_eq!(st.timing_pid, Some(AUDIO_PID));
    assert_eq!(st.carrier_pid, Some(AUDIO_PID));
    assert!(st.injected > 0, "election deadlocked: no PCR was injected");

    assert_monotone(&ev, AUDIO_PID);
    assert_lead(&ev, AUDIO_PID, AUDIO_PID, LEAD_OTHER, 150 * TICKS_MS);
    assert_cc_integrity(&ev);
}

// Probe: TopUp cadence at the stop-window boundary: with reals at a 560 ms
// period the last extrapolated injection before each real is clamped to the
// stop-window edge, keeping every same-era gap within 40 ms.
#[test]
fn topup_spacing_holds_at_stop_window() {
    let mut f = Fixture::new(VIDEO_PID);
    let mut reals = Vec::new();
    let mut next_pcr_ms = 0u64;
    for i in 0 .. 460 {
        f.av_frame(i, 90_000);
        let t_ms = i * 40;
        if t_ms >= next_pcr_ms {
            let dts = 90_000 + i * 3600;
            let v = expected_pcr(dts, 300 * TICKS_MS);
            f.pcr_packet(VIDEO_PID, v, false);
            reals.push(v);
            next_pcr_ms = t_ms + 540;
        }
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);
    let ev = events(&out);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::TopUp);
    assert!(st.injected > 0);
    assert_eq!(st.discontinuities, 0);

    // From the first injection on, every same-era gap honours the 40 ms
    // repetition bound, including the gap into each real PCR
    let pcrs = pcr_events(&ev, VIDEO_PID);
    let first = pcrs
        .iter()
        .find(|e| !reals.contains(&e.pcr.unwrap()))
        .expect("TopUp never injected")
        .index;
    assert_monotone(&ev, VIDEO_PID);
    assert_spacing(&ev, VIDEO_PID, 40 * TICKS_MS, first);
}

// Probe: a single-packet PMT section: the packet that completes a changed
// section passes verbatim exactly once, repetitions are patched.
#[test]
fn pmt_single_packet_first_sighting_verbatim() {
    let mut f = Fixture::new(0x1FFF); // single-packet PMT section
    for i in 0 .. 60 {
        f.av_frame(i, 90_000);
    }
    f.set_pmt(0x1FFF, 5, 0);
    for i in 60 .. 120 {
        f.av_frame(i, 90_000);
    }

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert!(st.pmt_patched);

    let versions = pmt_versions(&out);
    let len = versions[0].2;
    // Arming happens midway through the first upstream version: verbatim
    // sightings of version 1 first, then patched repetitions
    assert!(versions.contains(&(2, VIDEO_PID, len)));
    // The bumped upstream section passes verbatim exactly once and every
    // later repetition is patched with the new version + 1
    let raw_bumps = versions.iter().filter(|v| **v == (5, 0x1FFF, len)).count();
    assert_eq!(raw_bumps, 1, "changed section must pass verbatim exactly once");
    let bump = versions.iter().position(|v| v.0 == 5).unwrap();
    assert!(versions[bump + 1 ..].iter().all(|v| *v == (6, VIDEO_PID, len)));
    assert!(versions.len() > bump + 2);
}

// Probe: a short PMT section packed twice back-to-back in one packet: both
// copies are patched, so downstream never sees two versions of the program.
#[test]
fn pmt_packed_double_section_both_copies_patched() {
    let sections = pmt_sections(0x1FFF, 1, 0);
    let section: Vec<u8> = sections[0].to_vec();
    // pointer_field byte + two copies must fit into the 184-byte payload
    assert!(2 * section.len() < PACKET_SIZE - 4, "section too long to pack twice");
    let mut double = section.clone();
    double.extend_from_slice(&section);

    let mut f = Fixture::new(0x1FFF);
    f.pat.reset();
    let mut packet = [0u8; PACKET_SIZE];
    while f.pat.next(&mut packet) {
        f.push(packet);
    }

    let push_double = |f: &mut Fixture, cc: u8| {
        f.packets.push(pmt_packet_with_pointer(cc, 0, &[], &double, true));
    };

    push_double(&mut f, 0);
    for i in 0 .. 60 {
        f.video_frame(90_000 + i * 3600 + 7200, Some(90_000 + i * 3600), 700);
    }
    push_double(&mut f, 1);
    for i in 60 .. 70 {
        f.video_frame(90_000 + i * 3600 + 7200, Some(90_000 + i * 3600), 700);
    }
    push_double(&mut f, 2);

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert!(st.pmt_patched);

    let mut expected = section.clone();
    let mut pmt = PmtSectionMut::try_from(&mut expected[..]).unwrap();
    pmt.set_pcr_pid(VIDEO_PID);
    pmt.set_version(2);
    pmt.update_crc32();

    // The first sighting passes verbatim, later packets carry the patched
    // section in both copies
    let pmt_out: Vec<&[u8]> = out
        .chunks_exact(PACKET_SIZE)
        .filter(|c| (u16::from(c[1] & 0x1F) << 8) | u16::from(c[2]) == PMT_PID)
        .collect();
    assert_eq!(pmt_out.len(), 3);
    assert_eq!(
        &pmt_out[0][5 .. 5 + double.len()],
        &double[..],
        "first sighting must be verbatim"
    );
    for p in &pmt_out[1 ..] {
        let one = &p[5 .. 5 + section.len()];
        let two = &p[5 + section.len() .. 5 + double.len()];
        assert_eq!(one, &expected[..], "first copy not patched");
        assert_eq!(two, &expected[..], "second copy not patched");
    }
}

/// Reassembles PMT sections from output packets, keeping CRC failures.
fn pmt_sections_checked(out: &[u8]) -> Vec<Result<(u8, u16), ()>> {
    let mut psi = Psi::default();
    let mut result = Vec::new();
    for chunk in out.chunks_exact(PACKET_SIZE) {
        let arr: &[u8; PACKET_SIZE] = chunk.try_into().unwrap();
        let ts = TsPacketRef::from(arr);
        if ts.pid() != PMT_PID {
            continue;
        }
        psi.assemble(&ts);
        if let Some(section) = psi.sections().first() {
            result.push(
                PmtSectionRef::try_from(section)
                    .map(|pmt| (pmt.version(), pmt.pcr_pid()))
                    .map_err(|_| ()),
            );
        }
    }
    result
}

// Probe: upstream changes the PMT in the packet that carries the old
// section's tail plus the new section's start: the matching tail is still
// patched, so the in-flight repetition (patched head packets) completes
// with a valid CRC while the new section passes verbatim and re-learns.
#[test]
fn pmt_change_in_tail_plus_head_packet_keeps_crc() {
    let sections1 = pmt_sections(0x1FFF, 1, 204); // 230-byte section
    let section1: Vec<u8> = sections1[0].to_vec();
    let sections2 = pmt_sections(0x1FFF, 5, 204);
    let section2: Vec<u8> = sections2[0].to_vec();
    assert_eq!(section1.len(), 230);
    assert_eq!(section2.len(), 230);

    let mut f = Fixture::new(VIDEO_PID);
    f.pat.reset();
    let mut packet = [0u8; PACKET_SIZE];
    while f.pat.next(&mut packet) {
        f.push(packet);
    }

    // First sighting: plain two-packet repetition of section 1
    f.packets.push(pmt_packet_with_pointer(0, 0, &[], &section1[.. 183], true));
    f.packets.push(pmt_packet_with_pointer(1, 0, &[], &section1[183 ..], false));

    for i in 0 .. 60 {
        f.video_frame(90_000 + i * 3600 + 7200, Some(90_000 + i * 3600), 700);
    }

    // Repetition of section 1 whose closing packet carries the tail plus
    // the head of the changed section 2 via pointer_field
    f.packets.push(pmt_packet_with_pointer(2, 0, &[], &section1[.. 183], true));
    f.packets.push(pmt_packet_with_pointer(3, 47, &section1[183 ..], &section2[.. 136], true));
    f.packets.push(pmt_packet_with_pointer(4, 0, &[], &section2[136 ..], false));

    for i in 60 .. 70 {
        f.video_frame(90_000 + i * 3600 + 7200, Some(90_000 + i * 3600), 700);
    }

    // Plain repetition of section 2
    f.packets.push(pmt_packet_with_pointer(5, 0, &[], &section2[.. 183], true));
    f.packets.push(pmt_packet_with_pointer(6, 0, &[], &section2[183 ..], false));

    let mut s = PcrSynth::new(auto());
    let out = run(&mut s, &f.packets);

    let st = s.status();
    assert_eq!(st.phase, PcrSynthPhase::Full);
    assert!(st.pmt_patched);

    let checked = pmt_sections_checked(&out);
    assert_eq!(
        checked,
        vec![
            Ok((1, 0x1FFF)),    // first sighting of section 1: verbatim
            Ok((2, VIDEO_PID)), // in-flight repetition completes patched
            Ok((5, 0x1FFF)),    // first sighting of section 2: verbatim
            Ok((6, VIDEO_PID)), // repetition of section 2: patched
        ]
    );
}
