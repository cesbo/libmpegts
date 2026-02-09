mod data;

use mpegts::{
    psi::*,
    slicer::TsSlicer,
};

#[test]
fn test_parse_pmt() {
    let mut psi = Psi::default();
    TsSlicer::new().slice(data::PMT).for_each(|p| {
        psi.assemble(&p);
    });
    let pmt = PmtSectionRef::try_from(&psi).expect("Valid PMT section");

    assert_eq!(pmt.version(), 1);
    assert_eq!(pmt.pnr(), 50455);
    assert_eq!(pmt.pcr(), 2318);

    let mut items = pmt.items();

    let item = items.next().expect("First item").expect("Valid PMT item");
    assert_eq!(item.stream_type(), 2);
    assert_eq!(item.pid(), 2318);
    let mut descriptors = item.descriptors().expect("Video descriptors").into_iter();
    let desc = descriptors
        .next()
        .expect("First video descriptor")
        .expect("Valid descriptor");
    assert_eq!(desc.tag(), 0x0E);
    let desc = descriptors
        .next()
        .expect("Second video descriptor")
        .expect("Valid descriptor");
    assert_eq!(desc.tag(), 0x09);
    let desc = descriptors
        .next()
        .expect("Third video descriptor")
        .expect("Valid descriptor");
    assert_eq!(desc.tag(), 0x52);

    let item = items.next().expect("Second item").expect("Valid PMT item");
    assert_eq!(item.stream_type(), 4);
    assert_eq!(item.pid(), 2319);
    let mut descriptors = item.descriptors().expect("Audio descriptors").into_iter();
    let desc = descriptors
        .next()
        .expect("First audio descriptor")
        .expect("Valid descriptor");
    assert_eq!(desc.tag(), 0x0E);
    let desc = descriptors
        .next()
        .expect("Second audio descriptor")
        .expect("Valid descriptor");
    assert_eq!(desc.tag(), 0x0A);
    let desc = descriptors
        .next()
        .expect("Third audio descriptor")
        .expect("Valid descriptor");
    assert_eq!(desc.tag(), 0x52);
}
