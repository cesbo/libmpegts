use libmpegts::{
    pes::{
        PTS_MAX,
        PesHeader,
        PesPacketizer,
        STREAM_ID_AUDIO,
        STREAM_ID_VIDEO,
    },
    ts::{
        PACKET_SIZE,
        SYNC_BYTE,
        TsPacketRef,
    },
};

#[test]
fn test_pes_header_size() {
    // No PTS/DTS: 6 + 3 = 9 bytes
    let header = PesHeader::new(STREAM_ID_VIDEO);
    assert_eq!(header.size(), 9);

    // PTS only: 9 + 5 = 14 bytes
    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(0, None);
    assert_eq!(header.size(), 14);

    // PTS + DTS: 9 + 10 = 19 bytes
    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(0, Some(0));
    assert_eq!(header.size(), 19);
}

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
    let header = PesHeader::new(STREAM_ID_AUDIO).with_pts_dts(pts, None);
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
    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(pts, Some(dts));
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
    let pts = PTS_MAX;
    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(pts, None);
    let mut buf = [0u8; 32];
    header.write(&mut buf);

    let decoded_pts = decode_timestamp(&buf[9 .. 14]);
    assert_eq!(decoded_pts, pts);
}

#[test]
fn test_packetizer_single_packet() {
    let mut packetizer = PesPacketizer::new(0x100);

    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(90000, None);
    // Small payload that fits in one TS packet
    let payload = vec![0xAB; 100];

    packetizer.set_frame(&header, payload.clone());

    let mut packet = [0u8; PACKET_SIZE];
    assert!(packetizer.next(&mut packet));

    // Verify TS header
    assert_eq!(packet[0], SYNC_BYTE);
    assert_eq!((packet[1] & 0x40), 0x40, "PUSI should be set");

    // Verify PID
    let ts_ref = TsPacketRef::from(&packet);
    assert_eq!(ts_ref.pid(), 0x100);

    // 4 - TS header
    // 2 - AF length + flags
    let ts_header_size = 4 + 2;
    let pes_header_size = header.size();
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
fn test_packetizer_multiple_packets() {
    let mut packetizer = PesPacketizer::new(0x200);

    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(90000, None);

    // Large payload requiring multiple TS packets
    // (14 + 500) / 184 -> 3 TS packets
    let payload = vec![0xCD; 500];

    packetizer.set_frame(&header, payload);

    let mut packets = Vec::new();
    let mut packet = [0u8; PACKET_SIZE];
    while packetizer.next(&mut packet) {
        packets.push(packet);
    }

    assert_eq!(packets.len(), 3, "Should produce 3 packets");

    // First packet: PUSI=1
    assert_eq!(
        (packets[0][1] & 0x40),
        0x40,
        "First packet should have PUSI"
    );

    // Remaining packets: PUSI=0
    for (i, p) in packets[1 ..].iter().enumerate() {
        assert_eq!(
            (p[1] & 0x40),
            0x00,
            "No PUSI in continuation packet {}",
            i + 1
        );
    }

    // Last packet may have stuffing
    let last = &packets[2];
    let has_af = (last[3] & 0x20) != 0;
    if has_af {
        let af_length = last[4] as usize;
        if af_length > 1 {
            let stuffing_size = af_length - 1;
            for &b in &last[6 .. 6 + stuffing_size] {
                assert_eq!(b, 0xFF, "Stuffing bytes should be 0xFF");
            }
        }
    }
}

#[test]
fn test_packetizer_cc_wrap() {
    let mut packetizer = PesPacketizer::new(0x100);

    // Generate enough packets to wrap CC (0-15)
    for _ in 0 .. 20 {
        let header = PesHeader::new(STREAM_ID_VIDEO);
        packetizer.set_frame(&header, vec![0u8; 10]);

        let mut packet = [0u8; PACKET_SIZE];
        while packetizer.next(&mut packet) {}
    }

    // Generate one more and check CC value wrapped correctly
    let header = PesHeader::new(STREAM_ID_VIDEO);
    packetizer.set_frame(&header, vec![0u8; 10]);

    let mut packet = [0u8; PACKET_SIZE];
    packetizer.next(&mut packet);

    let ts_ref = TsPacketRef::from(&packet);
    // 20 single-packet frames → CC should be 20 % 16 = 4
    assert_eq!(ts_ref.cc(), 4, "CC should wrap at 16");
}

#[test]
fn test_packetizer_cc_continuous_across_frames() {
    let mut packetizer = PesPacketizer::new(0x100);

    let mut prev_cc: Option<u8> = None;

    for _ in 0 .. 5 {
        let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(90000, None);
        packetizer.set_frame(&header, vec![0xAA; 500]);

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
fn test_packetizer_pid() {
    let mut packetizer = PesPacketizer::new(0x1FFF); // Max valid PID

    let header = PesHeader::new(STREAM_ID_VIDEO);
    packetizer.set_frame(&header, vec![0u8; 10]);

    let mut packet = [0u8; PACKET_SIZE];
    packetizer.next(&mut packet);

    let ts_ref = TsPacketRef::from(&packet);
    assert_eq!(ts_ref.pid(), 0x1FFF);
}

#[test]
fn test_packetizer_set_frame_replaces() {
    let mut packetizer = PesPacketizer::new(0x100);

    // First frame
    let header = PesHeader::new(STREAM_ID_VIDEO);
    packetizer.set_frame(&header, vec![0xAA; 10]);

    let mut packet = [0u8; PACKET_SIZE];
    assert!(packetizer.next(&mut packet));
    assert!(
        !packetizer.next(&mut packet),
        "Should be only one packet for first frame"
    );

    let ts_ref = TsPacketRef::from(&packet);
    assert_eq!(ts_ref.cc(), 0);

    // Replace with second frame - CC should continue
    packetizer.set_frame(&header, vec![0xBB; 10]);
    assert!(packetizer.next(&mut packet));

    let ts_ref = TsPacketRef::from(&packet);
    assert_eq!(ts_ref.cc(), 1, "CC should continue after set_frame");
}

/// Helper function to decode 33-bit PTS/DTS from 5 bytes
fn decode_timestamp(buf: &[u8]) -> u64 {
    let b0 = ((buf[0] & 0x0E) >> 1) as u64;
    let b1 = buf[1] as u64;
    let b2 = ((buf[2] & 0xFE) >> 1) as u64;
    let b3 = buf[3] as u64;
    let b4 = ((buf[4] & 0xFE) >> 1) as u64;

    (b0 << 30) | (b1 << 22) | (b2 << 15) | (b3 << 7) | b4
}
