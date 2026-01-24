use mpegts::{
    pes::{
        PTS_MAX,
        PesHeader,
        PesPacketizer,
        PesPacketizerError,
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
    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts(0);
    assert_eq!(header.size(), 14);

    // PTS + DTS: 9 + 10 = 19 bytes
    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(0, 0);
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
    let header = PesHeader::new(STREAM_ID_AUDIO).with_pts(pts);
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
    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts_dts(pts, dts);
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
    let header = PesHeader::new(STREAM_ID_VIDEO).with_data_alignment();
    let mut buf = [0u8; 32];
    header.write(&mut buf);

    // Data alignment indicator is bit 2 of byte 6
    assert_eq!(buf[6] & 0x04, 0x04);
}

#[test]
fn test_pes_header_pts_max_value() {
    // Test with maximum 33-bit value
    let pts = PTS_MAX;
    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts(pts);
    let mut buf = [0u8; 32];
    header.write(&mut buf);

    let decoded_pts = decode_timestamp(&buf[9 .. 14]);
    assert_eq!(decoded_pts, pts);
}

#[test]
fn test_packetizer_single_packet() {
    let mut packetizer = PesPacketizer::new(0x100, 1024 * 1024);

    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts(90000);
    // Small payload that fits in one TS packet
    let payload = vec![0xAB; 100];

    packetizer.packetize(&header, &payload).unwrap();

    assert_eq!(packetizer.len(), PACKET_SIZE);
    assert!(!packetizer.is_empty());

    let data = packetizer.peek();
    assert_eq!(data.len(), PACKET_SIZE);
    let packet: [u8; PACKET_SIZE] = data[.. PACKET_SIZE].try_into().unwrap();
    packetizer.consume(PACKET_SIZE);

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

    assert!(packetizer.is_empty());
}

#[test]
fn test_packetizer_multiple_packets() {
    let mut packetizer = PesPacketizer::new(0x200, 1024 * 1024);

    let header = PesHeader::new(STREAM_ID_VIDEO).with_pts(90000);

    // Large payload requiring multiple TS packets
    // (14 + 500) / 188 -> 3 TS packets
    // 184 + 184 = 368, remaining 146 in last packet, stuffing 36 bytes
    let payload = vec![0xCD; 500];

    packetizer.packetize(&header, &payload).unwrap();

    assert_eq!(
        packetizer.len(),
        PACKET_SIZE * 3,
        "Should produce 3 packets"
    );
    let packet_count = packetizer.len() / PACKET_SIZE;

    // First packet: PUSI=1
    let first = pop_packet(&mut packetizer);
    assert_eq!((first[1] & 0x40), 0x40, "First packet should have PUSI");

    // Remaining packets: PUSI=0
    for _ in 1 .. packet_count {
        let packet = pop_packet(&mut packetizer);
        assert_eq!((packet[1] & 0x40), 0x00, "No PUSI in continuation packets");

        let is_last = packetizer.is_empty();
        if is_last {
            // Last packet may have stuffing
            let has_af = (packet[3] & 0x20) != 0;
            if has_af {
                let af_length = packet[4] as usize;
                if af_length > 1 {
                    let stuffing_size = af_length - 1;
                    for &b in &packet[6 .. 6 + stuffing_size] {
                        assert_eq!(b, 0xFF, "Stuffing bytes should be 0xFF");
                    }
                }
            }
        }
    }
}

#[test]
fn test_packetizer_cc_wrap() {
    let mut packetizer = PesPacketizer::new(0x100, 1024 * 1024);

    // Generate enough packets to wrap CC (0-15)
    for _ in 0 .. 20 {
        let header = PesHeader::new(STREAM_ID_VIDEO);
        packetizer.packetize(&header, &[0u8; 10]).unwrap();
    }

    // Pop all and check CC values
    let mut prev_cc: Option<u8> = None;
    while packetizer.len() >= PACKET_SIZE {
        let packet = pop_packet(&mut packetizer);
        let ts_ref = TsPacketRef::from(&packet);
        let cc = ts_ref.cc();
        if let Some(prev) = prev_cc {
            let expected = (prev + 1) & 0x0F;
            assert_eq!(cc, expected, "CC should increment and wrap at 16");
        }
        prev_cc = Some(cc);
    }
}

#[test]
fn test_packetizer_pid() {
    let mut packetizer = PesPacketizer::new(0x1FFF, 1024 * 1024); // Max valid PID

    let header = PesHeader::new(STREAM_ID_VIDEO);
    packetizer.packetize(&header, &[0u8; 10]).unwrap();

    let packet = pop_packet(&mut packetizer);
    let ts_ref = TsPacketRef::from(&packet);
    assert_eq!(ts_ref.pid(), 0x1FFF);
}

#[test]
fn test_packetizer_buffer_full() {
    // Small buffer that can only hold 2 packets
    let mut packetizer = PesPacketizer::new(0x100, PACKET_SIZE * 2);

    let header = PesHeader::new(STREAM_ID_VIDEO);
    // This should succeed (fits in 1 packet)
    packetizer.packetize(&header, &[0u8; 10]).unwrap();

    // This should also succeed (2nd packet)
    packetizer.packetize(&header, &[0u8; 10]).unwrap();

    // This should fail - buffer full
    let result = packetizer.packetize(&header, &[0u8; 10]);
    assert!(matches!(result, Err(PesPacketizerError::BufferFull { .. })));

    // Consume one packet
    packetizer.consume(PACKET_SIZE);

    // Now it should succeed again
    packetizer.packetize(&header, &[0u8; 10]).unwrap();
}

#[test]
fn test_packetizer_ring_buffer_wrap() {
    // Buffer of 400 bytes - can hold 2 full TS packets (376 bytes) with 24 bytes remaining
    let mut packetizer = PesPacketizer::new(0x100, 400);

    let header = PesHeader::new(STREAM_ID_VIDEO);

    // Add 2 TS packets with minimal payload
    packetizer.packetize(&header, &[0xAA; 10]).unwrap();
    assert_eq!(packetizer.len(), PACKET_SIZE); // 188 bytes

    packetizer.packetize(&header, &[0xBB; 10]).unwrap();
    assert_eq!(packetizer.len(), PACKET_SIZE * 2); // 376 bytes

    // Consume one packet - frees up space at the beginning
    let _ = pop_packet(&mut packetizer);
    assert_eq!(packetizer.len(), PACKET_SIZE);

    // Total free: 188 + 24 = 212 bytes

    // Add another packet - this will wrap around the ring buffer
    // 24 bytes in the end + 164 bytes at the start
    packetizer.packetize(&header, &[0xCC; 10]).unwrap();
    assert_eq!(packetizer.len(), PACKET_SIZE * 2); // 376 bytes total

    // peek() returns contiguous data from read position (212 bytes)
    let first_part = packetizer.peek();
    assert_eq!(
        first_part.len(),
        212,
        "First contiguous slice: 2nd packet + 24 bytes of 3rd"
    );
    assert_eq!(
        first_part[0], SYNC_BYTE,
        "Should start with SYNC_BYTE of 2nd packet"
    );
    assert_eq!(
        first_part[PACKET_SIZE], SYNC_BYTE,
        "24 bytes into slice should be SYNC_BYTE of 3rd packet"
    );
    packetizer.consume(first_part.len());

    // Now peek returns 164 bytes at buffer start (3rd packet tail)
    assert_eq!(packetizer.len(), 164); // 164 bytes remaining at buffer start
    let tail_part = packetizer.peek();
    assert_eq!(
        tail_part.len(),
        164,
        "164 bytes of 3rd packet at buffer start"
    );
    packetizer.consume(164);

    assert!(packetizer.is_empty());
    assert_eq!(packetizer.len(), 0);
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

// Helper to pop one packet
fn pop_packet(packetizer: &mut PesPacketizer) -> [u8; PACKET_SIZE] {
    let data = packetizer.peek();
    let packet: [u8; PACKET_SIZE] = data[.. PACKET_SIZE].try_into().unwrap();
    packetizer.consume(PACKET_SIZE);
    packet
}
