use libmpegts::{
    mux::{
        Multiplexer,
        MuxStream,
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

    // First drain emits PSI (psi_dirty)
    let mut buf = [0u8; PACKET_SIZE * 10];
    let n1 = mux.drain(&mut buf);
    assert_eq!(n1, PACKET_SIZE * 2);

    // Second drain — no longer dirty, no PSI
    let mut buf = [0u8; PACKET_SIZE * 10];
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
