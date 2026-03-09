use libmpegts::{
    mux::{
        Multiplexer,
        MuxFrame,
        MuxStream,
    },
    pes::{
        PtsDts,
        Timestamp,
    },
    ts::{
        PACKET_SIZE,
        TsPacketRef,
    },
};

fn packet_from_buf(buf: &[u8], offset: usize) -> TsPacketRef<'_> {
    let end = offset + PACKET_SIZE;
    TsPacketRef::from(<&[u8; PACKET_SIZE]>::try_from(&buf[offset .. end]).unwrap())
}

#[test]
fn test_emit_psi_pat_and_pmt() {
    let mut mux = Multiplexer::new(1);
    mux.set_service(1, 256, None);

    mux.add_stream(MuxStream::new(0x1B, 101));
    mux.add_stream(MuxStream::new(0x0F, 102));

    // drain() should auto-emit PSI because psi_dirty is set
    let mut buf = [0u8; PACKET_SIZE * 10];
    let n = mux.drain(&mut buf);

    // At least PAT + PMT = 2 packets
    assert_eq!(
        n,
        PACKET_SIZE * 2,
        "should emit exactly 2 packets for PAT and PMT"
    );

    // First packet should be PAT
    let pat_pkt = packet_from_buf(&buf, 0);
    assert!(pat_pkt.is_sync(), "PAT packet missing sync byte");
    assert_eq!(pat_pkt.pid(), 0x0000, "first packet should be PAT (PID 0)");
    assert!(pat_pkt.is_payload_start(), "PAT should have PUSI set");
    assert_eq!(
        pat_pkt.payload().unwrap()[1],
        0x00,
        "PAT table_id should be 0x00"
    );

    // Second packet should be PMT (PID = 256)
    let pmt_pkt = packet_from_buf(&buf, PACKET_SIZE);
    assert!(pmt_pkt.is_sync(), "PMT packet missing sync byte");
    assert_eq!(pmt_pkt.pid(), 256, "second packet should be PMT (PID 256)");
    assert!(pmt_pkt.is_payload_start(), "PMT should have PUSI set");
    assert_eq!(
        pmt_pkt.payload().unwrap()[1],
        0x02,
        "PMT table_id should be 0x02"
    );
}

#[test]
fn test_emit_psi_after_first_drain_not_dirty() {
    let mut mux = Multiplexer::new(1);
    mux.add_stream(MuxStream::new(0x1B, 101));

    let mut buf = [0u8; PACKET_SIZE * 10];

    // First drain emits PSI (psi_dirty)
    let n1 = mux.drain(&mut buf);
    assert_eq!(n1, PACKET_SIZE * 2);

    // Second drain — no longer dirty, no PSI
    let n2 = mux.drain(&mut buf);
    assert_eq!(n2, 0, "no PSI should be emitted when not dirty");
}

#[test]
fn test_emit_psi_small_buffer() {
    let mut mux = Multiplexer::new(1);
    mux.set_service(1, 256, None);
    mux.add_stream(MuxStream::new(0x1B, 101));

    let mut buf = [0u8; PACKET_SIZE];

    // Should have written exactly 1 PAT packet
    let n1 = mux.drain(&mut buf);
    assert_eq!(n1, PACKET_SIZE, "should emit PAT packet");
    let pkt = packet_from_buf(&buf, 0);
    assert_eq!(pkt.pid(), 0x0000, "first packet should be PAT");

    // Second drain should emit PMT
    let n2 = mux.drain(&mut buf);
    assert_eq!(n2, PACKET_SIZE, "should emit PMT packet");
    let pkt = packet_from_buf(&buf, 0);
    assert_eq!(pkt.pid(), 256, "next packet should be PMT");

    // Third drain should emit nothing (PSI already emitted)
    let n3 = mux.drain(&mut buf);
    assert_eq!(n3, 0, "no more PSI should be emitted");
}

/// Coefficient of Variation for inter-packet distances.
fn spacing_cv(positions: &[usize]) -> f64 {
    assert!(
        positions.len() >= 2,
        "need at least 2 packets to compute CV"
    );

    let gaps: Vec<f64> = positions.windows(2).map(|w| (w[1] - w[0]) as f64).collect();

    let n = gaps.len() as f64;
    let mean = gaps.iter().sum::<f64>() / n;
    let variance = gaps.iter().map(|g| (g - mean).powi(2)).sum::<f64>() / n;
    // standard deviation
    let sigma = variance.sqrt();

    sigma / mean
}

fn expected_packet_count(payload_len: usize) -> usize {
    let pes_header_len = 14;
    (pes_header_len + payload_len).div_ceil(PACKET_SIZE - 4)
}

#[test]
fn test_spacing_cv_dump() {
    // Simulate a "dumb" multiplexer that outputs frames without interleaving.
    // Video: 15000 bytes → 82 TS packets per frame, PTS interval = 3600
    // Audio: 1024 bytes → 6 TS packets per frame, PTS interval = 1920
    //
    // Pattern: [82V, 6A, 82V, 12A, 82V, 12A, ...]

    let video_per_frame = 82;
    let audio_per_frame = 6;

    let mut video_positions = Vec::new();
    let mut audio_positions = Vec::new();
    let mut offset = 0;
    let mut processed_audio = 0;

    for i in 0 .. 240 {
        video_positions.extend(offset .. offset + video_per_frame);
        offset += video_per_frame;

        let current_time = i * 3600;
        let total_audio = current_time / 1920;
        let audio_to_process = total_audio - processed_audio;
        let audio_count = audio_to_process * audio_per_frame;
        if audio_count > 0 {
            audio_positions.extend(offset .. offset + audio_count as usize);
            offset += audio_count as usize;
        }
        processed_audio = total_audio;
    }

    let cv_audio = spacing_cv(&audio_positions);
    let cv_video = spacing_cv(&video_positions);

    eprintln!("dumb mux — video CV: {cv_video:.4}, audio CV: {cv_audio:.4}");

    // Bursty output → high CV (bad interleaving)
    assert!(
        cv_audio > 0.5,
        "bursty audio should have CV > 0.5, got {cv_audio:.4}"
    );
}

#[test]
fn test_spacing_cv_uniform() {
    // Simulate ideal interleaving: audio packets evenly spread among video.

    let video_per_frame = 82;
    let audio_per_frame = 6;
    let total_per_frame = video_per_frame + audio_per_frame;

    let mut video_positions = Vec::new();
    let mut audio_positions = Vec::new();
    let mut offset = 0;

    for _ in 0 .. 240 {
        let mut audio_slots: Vec<usize> = (0 .. audio_per_frame)
            .map(|i| (i * total_per_frame) / audio_per_frame)
            .collect();

        for pos in 0 .. total_per_frame {
            if audio_slots.first() == Some(&pos) {
                audio_positions.push(offset + pos);
                audio_slots.remove(0);
            } else {
                video_positions.push(offset + pos);
            }
        }
        offset += total_per_frame;
    }

    let cv_audio = spacing_cv(&audio_positions);
    let cv_video = spacing_cv(&video_positions);

    eprintln!("uniform mux — video CV: {cv_video:.4}, audio CV: {cv_audio:.4}");

    // Evenly spread → low CV (good interleaving)
    assert!(
        cv_audio < 0.1,
        "uniform audio should have CV < 0.1, got {cv_audio:.4}"
    );
}

#[test]
fn test_whole_frame_scheduling() {
    let mut mux = Multiplexer::new(1);
    let video = mux.add_stream(MuxStream::new(0x1B, 101));
    let audio = mux.add_stream(MuxStream::new(0x0F, 102));

    mux.push_frame(
        video,
        MuxFrame::new(vec![0u8; 15_000]).with_pts_dts(PtsDts::new(0)),
    );
    mux.push_frame(
        video,
        MuxFrame::new(vec![0u8; 15_000]).with_pts_dts(PtsDts::new(3600)),
    );
    mux.push_frame(
        audio,
        MuxFrame::new(vec![0u8; 1024]).with_pts_dts(PtsDts::new(0)),
    );
    mux.push_frame(
        audio,
        MuxFrame::new(vec![0u8; 1024]).with_pts_dts(PtsDts::new(1920)),
    );

    let mut buf = vec![0u8; PACKET_SIZE * 512];
    let n = mux.drain(&mut buf);
    assert!(n > 0, "drain should produce output");

    let mut runs = Vec::new();
    let mut current_run: Option<(u16, usize)> = None;

    for i in 0 .. (n / PACKET_SIZE) {
        let pkt = packet_from_buf(&buf, i * PACKET_SIZE);
        match pkt.pid() {
            101 | 102 if pkt.payload().is_some() => {
                if let Some((pid, len)) = current_run {
                    if pid == pkt.pid() {
                        current_run = Some((pid, len + 1));
                    } else {
                        runs.push((pid, len));
                        current_run = Some((pkt.pid(), 1));
                    }
                } else {
                    current_run = Some((pkt.pid(), 1));
                }
            }
            _ => {}
        }
    }

    if let Some(run) = current_run {
        runs.push(run);
    }

    assert_eq!(
        runs,
        vec![
            (101, expected_packet_count(15_000)),
            (102, expected_packet_count(1024) * 2),
            (101, expected_packet_count(15_000)),
        ],
        "stream packets should be emitted as contiguous frame-sized runs"
    );
}

#[test]
fn test_whole_frame_scheduling_across_timestamp_wrap() {
    let mut mux = Multiplexer::new(1);
    let video = mux.add_stream(MuxStream::new(0x1B, 101));
    let audio = mux.add_stream(MuxStream::new(0x0F, 102));

    mux.push_frame(
        video,
        MuxFrame::new(vec![0u8; 512]).with_pts_dts(PtsDts::new(Timestamp::MAX - 100)),
    );
    mux.push_frame(
        audio,
        MuxFrame::new(vec![0u8; 512]).with_pts_dts(PtsDts::new(50)),
    );

    let mut buf = vec![0u8; PACKET_SIZE * 64];
    let n = mux.drain(&mut buf);
    assert!(n > 0, "drain should produce output");

    let first_es_pid = (0 .. (n / PACKET_SIZE)).find_map(|i| {
        let pkt = packet_from_buf(&buf, i * PACKET_SIZE);
        match pkt.pid() {
            101 | 102 if pkt.payload().is_some() => Some(pkt.pid()),
            _ => None,
        }
    });

    assert_eq!(
        first_es_pid,
        Some(101),
        "pre-wrap timestamp must be emitted before wrapped timestamp"
    );
}
