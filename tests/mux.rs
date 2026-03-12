use libmpegts::{
    mux::{
        Multiplexer,
        MuxFrame,
    },
    psi::PmtStream,
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
fn test_emit_psi() {
    let mut mux = Multiplexer::new(1);
    mux.set_service(1, 256, None);
    let video = mux.add_stream(PmtStream {
        stream_type: 0x1B,
        elementary_pid: 101,
        stream_descriptors: Vec::new(),
    });
    let _audio = mux.add_stream(PmtStream {
        stream_type: 0x0F,
        elementary_pid: 102,
        stream_descriptors: Vec::new(),
    });
    mux.push_frame(
        video,
        MuxFrame {
            data: vec![0u8; 100],
            is_key_frame: true,
            pts_dts: Some((0, None).into()),
        },
    );

    // drain() should auto-emit PSI because psi_dirty is set
    let mut buf = [0u8; PACKET_SIZE * 10];
    let n = mux.drain(&mut buf);

    // At least PAT + PMT = 2 packets
    assert!(
        n >= PACKET_SIZE * 2,
        "should emit at least 2 packets for PAT and PMT"
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
fn test_emit_psi_small_buffer() {
    let mut mux = Multiplexer::new(1);
    mux.set_service(1, 256, None);
    let video = mux.add_stream(PmtStream {
        stream_type: 0x1B,
        elementary_pid: 101,
        stream_descriptors: Vec::new(),
    });
    mux.push_frame(
        video,
        MuxFrame {
            data: vec![0u8; 100],
            is_key_frame: true,
            pts_dts: Some((0, None).into()),
        },
    );

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

/// Simulate a "dumb" multiplexer that outputs frames without interleaving.
/// Video: 15000 bytes → 82 TS packets per frame, PTS interval = 3600
/// Audio: 1024 bytes → 6 TS packets per frame, PTS interval = 1920
///
/// Pattern: [82V, 6A, 82V, 12A, 82V, 12A, ...]
#[test]
fn test_spacing_cv_dumb() {
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

/// Simulate ideal interleaving: audio packets evenly spread among video.
#[test]
fn test_spacing_cv_uniform() {
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
#[ignore]
fn test_interleaving_cv() {
    let mut mux = Multiplexer::new(1);
    let video = mux.add_stream(PmtStream {
        stream_type: 0x1B,
        elementary_pid: 101,
        stream_descriptors: Vec::new(),
    });
    let audio = mux.add_stream(PmtStream {
        stream_type: 0x0F,
        elementary_pid: 102,
        stream_descriptors: Vec::new(),
    });

    // 240 video frames, 15000 bytes each, GOP=30
    for i in 0 .. 240u64 {
        let pts = i * 3600;
        mux.push_frame(
            video,
            MuxFrame {
                data: vec![0u8; 15_000],
                is_key_frame: i % 30 == 0,
                pts_dts: Some((pts, None).into()),
            },
        );
    }

    // 450 audio frames, 1024 bytes each
    for i in 0 .. 450u64 {
        let pts = i * 1920;
        mux.push_frame(
            audio,
            MuxFrame {
                data: vec![0u8; 1024],
                is_key_frame: false,
                pts_dts: Some((pts, None).into()),
            },
        );
    }

    // Upper bound: video ~19200 + audio ~2700 + PSI/PCR ~600
    let mut buf = vec![0u8; PACKET_SIZE * 25_000];
    let n = mux.drain(&mut buf);
    assert!(n > 0, "drain should produce output");

    let total_packets = n / PACKET_SIZE;

    // Collect packet indices per PID
    let mut video_positions = Vec::new();
    let mut audio_positions = Vec::new();

    for i in 0 .. total_packets {
        let pkt = packet_from_buf(&buf, i * PACKET_SIZE);
        match pkt.pid() {
            101 => video_positions.push(i),
            102 => audio_positions.push(i),
            _ => {}
        }
    }

    eprintln!("total packets: {total_packets}");
    eprintln!("video packets: {}", video_positions.len());
    eprintln!("audio packets: {}", audio_positions.len());

    let cv_video = (video_positions.len() >= 2)
        .then(|| spacing_cv(&video_positions))
        .unwrap_or(1.0);
    let cv_audio = (audio_positions.len() >= 2)
        .then(|| spacing_cv(&audio_positions))
        .unwrap_or(1.0);

    eprintln!("video CV: {cv_video:.4}");
    eprintln!("audio CV: {cv_audio:.4}");

    assert!(
        cv_audio < 0.1,
        "audio inter-packet CV should be < 0.1, got {cv_audio:.4}"
    );
}
