use libmpegts::{
    psi::*,
    slicer::TsSlicer,
    ts::PACKET_SIZE,
};

#[test]
fn test_build_bat_roundtrip() {
    // No captured BAT exists: build one, packetize it on the SDT PID and
    // parse it back through TsSlicer + Psi::assemble.
    let mut descriptors = Vec::new();
    Desc41 {
        items: &[
            Desc41Item {
                service_id: 101,
                service_type: 1,
            },
            Desc41Item {
                service_id: 102,
                service_type: 2,
            },
        ],
    }
    .encode(&mut descriptors)
    .unwrap();

    let streams = (0 .. 40)
        .map(|i| BatStream {
            transport_stream_id: 1000 + i,
            original_network_id: 85,
            descriptors: descriptors.clone(),
        })
        .collect();

    let sections = BatBuilder::build(BatConfig {
        bouquet_id: 0x1001,
        version: 5,
        bouquet_descriptors: vec![0x47, 3, b'B', b'A', b'T'],
        streams,
    });
    assert_eq!(sections.len(), 1);

    let mut packetizer = PsiPacketizer::new(SDT_PID);
    packetizer.set_sections(sections);
    let mut ts = Vec::new();
    let mut packet = [0u8; PACKET_SIZE];
    while packetizer.next(&mut packet) {
        ts.extend_from_slice(&packet);
    }
    assert!(ts.len() > PACKET_SIZE);

    let mut psi = Psi::default();
    TsSlicer::new().slice(&ts).for_each(|p| {
        assert_eq!(p.pid(), SDT_PID);
        psi.assemble(&p);
    });
    let section = psi.sections().first().expect("BAT section expected");
    let bat = BatSectionRef::try_from(section).expect("Valid BAT section");

    assert_eq!(bat.table_id(), BAT_TABLE_ID);
    assert_eq!(bat.version(), 5);
    assert_eq!(bat.bouquet_id(), 0x1001);

    let desc = bat
        .bouquet_descriptors()
        .expect("Bouquet descriptors")
        .into_iter()
        .next()
        .expect("Bouquet name descriptor")
        .expect("Valid descriptor");
    assert_eq!(desc.tag(), 0x47);
    assert_eq!(desc.data(), b"BAT");

    let mut count = 0;
    for ts in bat.transport_streams() {
        let ts = ts.expect("Valid BAT transport stream");
        assert_eq!(ts.transport_stream_id(), 1000 + count);
        assert_eq!(ts.original_network_id(), 85);

        let desc = ts
            .transport_stream_descriptors()
            .expect("Transport stream descriptors")
            .into_iter()
            .next()
            .expect("Service list descriptor")
            .expect("Valid descriptor");
        let services: Vec<Desc41Item> = Desc41Ref::try_from(desc).unwrap().items().collect();
        assert_eq!(services.len(), 2);
        assert_eq!(services[0].service_id, 101);
        count += 1;
    }
    assert_eq!(count, 40);
}
