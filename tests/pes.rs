use libmpegts::{
    pes::{
        EsFrame,
        PesHeader,
        PesHeaderError,
        PesHeaderRef,
        PesPacketizer,
        PtsDts,
        STREAM_ID_AUDIO,
        STREAM_ID_VIDEO,
        Timestamp,
    },
    ts::{
        PACKET_SIZE,
        SYNC_BYTE,
        TsPacketRef,
    },
};

#[test]
fn test_pes_header_write_no_timestamp() {
    let header = PesHeader::new(STREAM_ID_VIDEO);
    let mut buf = [0u8; 32];
    let written = header.write(&mut buf);

    assert_eq!(written, 9);

    // Start code prefix
    assert_eq!(buf[0], 0x00);
    assert_eq!(buf[1], 0x00);
    assert_eq!(buf[2], 0x01);

    // Stream ID
    assert_eq!(buf[3], STREAM_ID_VIDEO);

    // PES packet length (0 = unbounded)
    assert_eq!(buf[4], 0x00);
    assert_eq!(buf[5], 0x00);

    // Flags: '10' marker
    assert_eq!(buf[6] & 0xC0, 0x80);

    // PTS/DTS flags: none
    assert_eq!(buf[7] >> 6, 0b00);

    // Header data length
    assert_eq!(buf[8], 0);
}

#[test]
fn test_pes_header_write_pts_only() {
    let pts = 90000u64; // 1 second at 90kHz
    let header = PesHeader::new(STREAM_ID_AUDIO).with_pts_dts(PtsDts::new(pts));
    let mut buf = [0u8; 32];
    let written = header.write(&mut buf);

    assert_eq!(written, 14);

    // PTS/DTS flags: PTS only
    assert_eq!(buf[7] >> 6, 0b10);

    // Header data length
    assert_eq!(buf[8], 5);

    // Decode PTS back
    let decoded_pts = decode_timestamp(&buf[9 .. 14]);
    assert_eq!(decoded_pts, pts);
}

#[test]
fn test_pes_header_write_pts_dts() {
    let pts = 180000u64; // 2 seconds
    let dts = 90000u64; // 1 second
    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(PtsDts::new(pts).with_dts(dts));
    let mut buf = [0u8; 32];
    let written = header.write(&mut buf);

    assert_eq!(written, 19);

    // PTS/DTS flags: both
    assert_eq!(buf[7] >> 6, 0b11);

    // Header data length
    assert_eq!(buf[8], 10);

    // Decode PTS
    let decoded_pts = decode_timestamp(&buf[9 .. 14]);
    assert_eq!(decoded_pts, pts);

    // Decode DTS
    let decoded_dts = decode_timestamp(&buf[14 .. 19]);
    assert_eq!(decoded_dts, dts);
}

#[test]
fn test_pes_header_data_alignment() {
    let header = PesHeader::new(STREAM_ID_VIDEO).with_data_alignment(true);
    let mut buf = [0u8; 32];
    header.write(&mut buf);

    // Data alignment indicator is bit 2 of byte 6
    assert_eq!(buf[6] & 0x04, 0x04);
}

#[test]
fn test_pes_header_pts_max_value() {
    // Test with maximum 33-bit value
    let pts = Timestamp::MAX;
    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(PtsDts::new(pts));
    let mut buf = [0u8; 32];
    header.write(&mut buf);

    let decoded_pts = decode_timestamp(&buf[9 .. 14]);
    assert_eq!(decoded_pts, pts);
}

#[test]
fn test_pes_header_ref_no_timestamp() {
    let header = PesHeader::new(STREAM_ID_VIDEO);
    let mut buf = [0u8; 32];
    let written = header.write(&mut buf);

    let header_ref = PesHeaderRef::try_from(&buf[.. written]).unwrap();

    assert_eq!(header_ref.stream_id(), STREAM_ID_VIDEO);
    assert_eq!(header_ref.packet_length(), 0);
    assert_eq!(header_ref.header_len(), written);
    assert_eq!(header_ref.pts_dts(), None);
    assert_eq!(header_ref.as_ref(), &buf[.. written]);
}

#[test]
fn test_pes_header_ref_pts_only() {
    let pts = 90000u64;
    let header = PesHeader::new(STREAM_ID_AUDIO).with_pts_dts(PtsDts::new(pts));
    let payload = [0xAB; 4];
    let mut buf = [0u8; 36];
    let written = header.write(&mut buf);
    buf[written .. written + payload.len()].copy_from_slice(&payload);

    let header_ref = PesHeaderRef::try_from(&buf[.. written + payload.len()]).unwrap();
    let pts_dts = header_ref.pts_dts().unwrap();

    assert_eq!(header_ref.stream_id(), STREAM_ID_AUDIO);
    assert_eq!(header_ref.header_len(), written);
    assert_eq!(pts_dts.pts.value(), pts);
    assert_eq!(pts_dts.dts, None);
}

#[test]
fn test_pes_header_ref_pts_dts() {
    let pts = 180000u64;
    let dts = 90000u64;
    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(PtsDts::new(pts).with_dts(dts));
    let mut buf = [0u8; 32];
    let written = header.write(&mut buf);

    let pts_dts = PesHeaderRef::try_from(&buf[.. written])
        .unwrap()
        .pts_dts()
        .unwrap();

    assert_eq!(pts_dts.pts.value(), pts);
    assert_eq!(pts_dts.dts.unwrap().value(), dts);
}

#[test]
fn test_pes_header_ref_rejects_invalid_header() {
    assert_eq!(
        PesHeaderRef::try_from(&[0x00, 0x00, 0x01][..]),
        Err(PesHeaderError::InvalidHeaderLength)
    );

    let mut buf = [0u8; 32];
    let written = PesHeader::new(STREAM_ID_VIDEO)
        .with_pts_dts(PtsDts::new(90000))
        .write(&mut buf);

    buf[2] = 0x02;
    assert_eq!(
        PesHeaderRef::try_from(&buf[.. written]),
        Err(PesHeaderError::InvalidStartCode)
    );
}

#[test]
fn test_packetizer_single_packet() {
    let mut packetizer = PesPacketizer::new(0x100);

    let mut packet = [0u8; PACKET_SIZE];
    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(PtsDts::new(90000));

    // Small payload that fits in one TS packet
    let payload = vec![0xAB; 100];
    let frame = EsFrame {
        header: header.clone(),
        payload: payload.clone(),
        rai: false,
    };
    packetizer.set_frame(frame);
    assert!(packetizer.next(&mut packet));

    // Verify TS header
    assert_eq!(packet[0], SYNC_BYTE);
    assert_eq!((packet[1] & 0x40), 0x40, "PUSI should be set");

    // Verify PID
    let ts_ref = TsPacketRef::from(&packet);
    assert_eq!(ts_ref.pid(), 0x100);

    // 4 (TS header) + 2 (AF length + flags)
    let ts_header_size = 4 + 2;
    // 9 (PES header) + 5 (PTS)
    let pes_header_size = 9 + 5;
    let stuffing_size = PACKET_SIZE - (ts_header_size + pes_header_size + payload.len());
    let payload_start = ts_header_size + stuffing_size + pes_header_size;

    // Verify adaptation field and stuffing
    let has_af = (packet[3] & 0x20) != 0;
    assert!(has_af, "Adaptation field should be present for stuffing");
    assert_eq!(packet[4] as usize, stuffing_size + 1);
    for &b in &packet[ts_header_size .. ts_header_size + stuffing_size] {
        assert_eq!(b, 0xFF, "Stuffing bytes should be 0xFF");
    }

    // Verify payload
    let has_payload = (packet[3] & 0x10) != 0;
    assert!(has_payload, "Payload flag should be set");
    assert_eq!(&packet[payload_start ..], payload.as_slice());

    // No more packets
    assert!(!packetizer.next(&mut packet));
}

#[test]
fn test_packetizer_single_packet_with_rai() {
    let mut packetizer = PesPacketizer::new(0x100);

    let mut packet = [0u8; PACKET_SIZE];
    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(PtsDts::new(90000));

    // Without AF stuffing
    // 14 (PES header) + 168 (payload) + 2 (AF for RAI) = 184
    // AF is exactly 2 bytes: length byte + flags byte (RAI), no stuffing
    let frame = EsFrame {
        header: header.clone(),
        payload: vec![0xAB; 168],
        rai: true,
    };
    packetizer.set_frame(frame);

    assert!(packetizer.next(&mut packet));
    assert!(!packetizer.next(&mut packet), "Should fit in one packet");

    // AF present with length = 1 (flags byte only, no stuffing)
    let has_af = (packet[3] & 0x20) != 0;
    assert!(has_af, "AF should be present for RAI");
    assert_eq!(packet[4], 1, "AF length should be 1 (flags byte only)");

    // RAI flag set
    assert_eq!(packet[5] & 0x40, 0x40, "RAI should be set");

    // With AF stuffing
    // 14 (PES header) + 100 (payload) + 2 (AF for RAI) + 68 (AF stuffing) = 184
    let frame = EsFrame {
        header,
        payload: vec![0xAB; 100],
        rai: true,
    };
    packetizer.set_frame(frame);

    assert!(packetizer.next(&mut packet));
    assert!(!packetizer.next(&mut packet), "Should fit in one packet");

    // AF present with length = 69 (1 byte for RAI flags + 68 bytes stuffing)
    let has_af = (packet[3] & 0x20) != 0;
    assert!(has_af, "AF should be present for RAI with stuffing");
    assert_eq!(
        packet[4], 69,
        "AF length should be 69 (1 for RAI flags + 68 stuffing)"
    );

    // RAI flag set
    assert_eq!(packet[5] & 0x40, 0x40, "RAI should be set with stuffing");
}

#[test]
fn test_packetizer_multiple_packets() {
    let mut packetizer = PesPacketizer::new(0x200);

    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(PtsDts::new(90000));

    // Large payload (500 bytes) requiring multiple TS packets
    // 1 TS: 14 (PES header) + 170 (payload)
    // 2 TS: 0 (PES header) + 184 (payload)
    // 3 TS: 0 (PES header) + 146 (payload) + 38 (AF length + stuffing)
    let payload = vec![0xCD; 500];

    let frame = EsFrame {
        header,
        payload,
        rai: false,
    };

    packetizer.set_frame(frame);

    let mut packet = [0u8; PACKET_SIZE];

    // First packet: PUSI=1
    assert!(packetizer.next(&mut packet));
    let ts_ref = TsPacketRef::from(&packet);
    assert!(ts_ref.is_payload_start(), "First packet should have PUSI");
    assert_eq!(ts_ref.cc(), 0, "CC should start at 0");

    // Remaining packets: PUSI=0
    assert!(packetizer.next(&mut packet));
    let ts_ref = TsPacketRef::from(&packet);
    assert!(!ts_ref.is_payload_start(), "No PUSI in continuation packet");
    assert_eq!(ts_ref.cc(), 1, "CC should increment to 1");

    // Last packet with stuffing
    assert!(packetizer.next(&mut packet));
    let ts_ref = TsPacketRef::from(&packet);
    assert!(!ts_ref.is_payload_start(), "No PUSI in last packet");
    assert_eq!(ts_ref.cc(), 2, "CC should increment to 2");

    let has_af = (packet[3] & 0x20) != 0;
    assert!(has_af, "AF should be present in last packet for stuffing");

    let af_length = packet[4] as usize;
    assert_eq!(af_length, 37, "AF length should be 37");

    assert_eq!(packet[5], 0, "No flags should be set in AF");

    let stuffing_size = af_length - 1;
    for &b in &packet[6 .. 6 + stuffing_size] {
        assert_eq!(b, 0xFF, "Stuffing bytes should be 0xFF");
    }

    // No more packets
    assert!(!packetizer.next(&mut packet));
}

#[test]
fn test_packetizer_multiple_packets_with_rai() {
    let mut packetizer = PesPacketizer::new(0x200);

    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(PtsDts::new(90000));
    let payload = vec![0xCD; 500];

    let frame = EsFrame {
        header,
        payload,
        rai: true,
    };

    packetizer.set_frame(frame);

    // Large payload (500 bytes) requiring multiple TS packets
    // 1 TS: 14 (PES header) 2 (AF length + flags with RAI) + 168 (payload)
    // 2 TS: 0 (PES header) + 184 (payload)
    // 3 TS: 0 (PES header) + 148 (payload) + 36 (AF length + stuffing)
    let mut packet = [0u8; PACKET_SIZE];

    // First packet: AF with RAI
    assert!(packetizer.next(&mut packet));
    assert!(packet[3] & 0x20 != 0, "First packet should have AF");
    assert_eq!(packet[4], 1, "AF length should be 1 for RAI");
    assert_eq!(packet[5], 0x40, "First packet should have RAI");

    // Second packet: no AF, no RAI
    assert!(packetizer.next(&mut packet));
    assert!(packet[3] & 0x20 == 0, "Second packet should not have AF");

    // Third packet: AF for stuffing, no RAI
    assert!(packetizer.next(&mut packet));
    assert!(
        packet[3] & 0x20 != 0,
        "Third packet should have AF for stuffing"
    );
    assert_eq!(packet[4], 35, "Third packet AF length should be 35");
    assert_eq!(packet[5], 0, "Third packet should not have flags");

    // No more packets
    assert!(!packetizer.next(&mut packet));
}

#[test]
fn test_packetizer_cc_wrap() {
    let mut packetizer = PesPacketizer::new(0x100);

    // Generate enough packets to wrap CC (0-15)
    for i in 0 .. 20 {
        let header = PesHeader::new(STREAM_ID_VIDEO);
        let payload = vec![0u8; 10];
        let frame = EsFrame {
            header,
            payload,
            rai: false,
        };
        packetizer.set_frame(frame);

        let mut packet = [0u8; PACKET_SIZE];
        assert!(packetizer.next(&mut packet));
        assert!(!packetizer.next(&mut packet));
        let ts_ref = TsPacketRef::from(&packet);
        assert_eq!(
            ts_ref.cc(),
            (i % 16) as u8,
            "CC should increment and wrap at 16"
        );
    }
}

#[test]
fn test_packetizer_cc_continuous_across_frames() {
    let mut packetizer = PesPacketizer::new(0x100);

    let mut prev_cc: Option<u8> = None;

    for _ in 0 .. 5 {
        let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(PtsDts::new(90000));
        let payload = vec![0xAA; 500];
        let frame = EsFrame {
            header,
            payload,
            rai: false,
        };
        packetizer.set_frame(frame);

        let mut packet = [0u8; PACKET_SIZE];
        while packetizer.next(&mut packet) {
            let ts_ref = TsPacketRef::from(&packet);
            let cc = ts_ref.cc();
            if let Some(prev) = prev_cc {
                let expected = (prev + 1) & 0x0F;
                assert_eq!(cc, expected, "CC should be continuous across frames");
            }
            prev_cc = Some(cc);
        }
    }
}

#[test]
fn test_packetizer_cc_around_pcr_packet() {
    let mut packet = [0u8; PACKET_SIZE];
    let mut packetizer = PesPacketizer::new(0x100);

    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(PtsDts::new(90000));
    let frame = EsFrame {
        header,
        payload: vec![0xAB; 300], // fits in 2 TS packets
        rai: false,
    };
    packetizer.set_frame(frame);

    assert!(packetizer.next(&mut packet));
    assert_eq!(TsPacketRef::from(&packet).cc(), 0, "first payload CC");

    assert!(packetizer.next(&mut packet));
    assert_eq!(TsPacketRef::from(&packet).cc(), 1, "second payload CC");

    // AF-only PCR packet on the same PID
    packetizer.build_pcr_packet(&mut packet, 90000 * 300);

    // adaptation_field_control must be '10'
    assert_eq!((packet[3] & 0x30), 0x20, "expected AF only, no payload");

    // PCR flag in the AF
    assert_eq!(packet[5] & 0x10, 0x10, "PCR flag must be set in AF");

    // CC of AF-only packet must equal last payload CC (1)
    assert_eq!(
        TsPacketRef::from(&packet).cc(),
        1,
        "AF-only PCR packet must repeat the last payload CC",
    );

    // Next payload packet should increment from the last payload CC (1 -> 2)
    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(PtsDts::new(180000));
    let frame = EsFrame {
        header,
        payload: vec![0xCD; 100],
        rai: false,
    };
    packetizer.set_frame(frame);

    assert!(packetizer.next(&mut packet));
    assert_eq!(TsPacketRef::from(&packet).cc(), 2, "third payload CC",);
}

#[test]
fn test_packetizer_pid() {
    let mut packetizer = PesPacketizer::new(8190);

    let header = PesHeader::new(STREAM_ID_VIDEO);
    let payload = vec![0u8; 10];
    let frame = EsFrame {
        header,
        payload,
        rai: false,
    };
    packetizer.set_frame(frame);

    let mut packet = [0u8; PACKET_SIZE];
    packetizer.next(&mut packet);

    let ts_ref = TsPacketRef::from(&packet);
    assert_eq!(ts_ref.pid(), 8190);
}

/// Helper function to decode 33-bit PTS/DTS from 5 bytes
#[test]
fn test_timestamp_wrapping() {
    // Normal addition
    let ts = Timestamp::new(1000);
    assert_eq!(ts.wrapping_add(500.into()).value(), 1500);

    // Normal subtraction
    assert_eq!(ts.wrapping_sub(400.into()).value(), 600);

    // Addition wraps at 2^33
    let ts = Timestamp::new(Timestamp::MAX);
    assert_eq!(ts.wrapping_add(1.into()).value(), 0);
    assert_eq!(ts.wrapping_add(100.into()).value(), 99);

    // Subtraction wraps at 2^33
    let ts = Timestamp::new(0);
    assert_eq!(ts.wrapping_sub(1.into()).value(), Timestamp::MAX);
    assert_eq!(ts.wrapping_sub(100.into()).value(), Timestamp::MAX - 99);
}

#[test]
fn test_timestamp_read() {
    let timestamp = Timestamp::new(90000);
    let mut buf = [0u8; 5];
    timestamp.write(&mut buf, 0b0010);

    assert_eq!(Timestamp::read(&buf, 0b0010), Some(timestamp));
    assert_eq!(Timestamp::read(&buf, 0b0011), None);

    buf[4] &= !0x01;
    assert_eq!(Timestamp::read(&buf, 0b0010), None);
}

fn decode_timestamp(buf: &[u8]) -> u64 {
    let b0 = ((buf[0] & 0x0E) >> 1) as u64;
    let b1 = buf[1] as u64;
    let b2 = ((buf[2] & 0xFE) >> 1) as u64;
    let b3 = buf[3] as u64;
    let b4 = ((buf[4] & 0xFE) >> 1) as u64;

    (b0 << 30) | (b1 << 22) | (b2 << 15) | (b3 << 7) | b4
}
