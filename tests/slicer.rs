mod data;

use libmpegts::{
    slicer::TsSlicer,
    ts::PACKET_SIZE,
};

#[test]
fn test_single_packet() {
    let mut slicer = TsSlicer::new();

    let packet = slicer.slice(data::PAT).next().expect("One packet expected");
    assert_eq!(packet.pid(), 0);
    assert_eq!(packet.as_ref(), &data::PAT);
}

#[test]
fn test_single_packet_partial() {
    let mut slicer = TsSlicer::new();

    assert!(slicer.slice(&data::PAT[.. 60]).next().is_none());
    assert!(slicer.slice(&data::PAT[60 .. 120]).next().is_none());
    assert!(slicer.slice(&data::PAT[120 .. 180]).next().is_none());
    let packet = slicer
        .slice(&data::PAT[180 ..])
        .next()
        .expect("One packet expected");
    assert_eq!(packet.pid(), 0);
    assert_eq!(packet.as_ref(), data::PAT);
}

#[test]
fn test_multiple_packets() {
    let mut slicer = TsSlicer::new();

    let mut count = 0;

    for packet in slicer.slice(data::EIT_50) {
        assert_eq!(packet.pid(), 0x12);
        let offset = count * PACKET_SIZE;
        assert_eq!(
            packet.as_ref(),
            &data::EIT_50[offset .. offset + PACKET_SIZE]
        );
        count += 1;
    }
    assert_eq!(count, data::EIT_50.len() / PACKET_SIZE);
}

#[test]
fn test_skip_trash() {
    let mut slicer = TsSlicer::new();
    let tmp = [&[0xFF, 0xFF, 0xFF, 0xFF], data::PAT].concat();
    let packet = slicer.slice(&tmp).next().expect("One packet expected");
    assert_eq!(packet.as_ref(), data::PAT);
    // check again
    let packet = slicer.slice(data::PAT).next().expect("One packet expected");
    assert_eq!(packet.as_ref(), data::PAT);
}
