extern crate mpegts;

mod data;
use mpegts::psi::{Psi, Pmt, Descriptor};


#[test]
fn test_parse_pmt() {
    let mut psi = Psi::default();
    psi.mux(&data::PMT);
    assert!(psi.check());

    let mut pmt = Pmt::default();
    pmt.parse(&psi);

    assert_eq!(pmt.version, 1);
    assert_eq!(pmt.pnr, 50455);
    assert_eq!(pmt.pcr, 2318);
    assert_eq!(pmt.descriptors.len(), 0);

    let item = &pmt.items[0];
    assert_eq!(item.stream_type, 2);
    assert_eq!(item.pid, 2318);
    let mut descriptors = item.descriptors.iter();
    match &descriptors.next().unwrap() {
        Descriptor::Desc0E(v) => v,
        _ => unreachable!()
    };
    match &descriptors.next().unwrap() {
        Descriptor::Desc09(v) => v,
        _ => unreachable!()
    };
    match &descriptors.next().unwrap() {
        Descriptor::Desc52(v) => v,
        _ => unreachable!()
    };

    let item = &pmt.items[1];
    assert_eq!(item.stream_type, 4);
    assert_eq!(item.pid, 2319);
    let mut descriptors = item.descriptors.iter();
    match &descriptors.next().unwrap() {
        Descriptor::Desc0E(v) => v,
        _ => unreachable!()
    };
    match &descriptors.next().unwrap() {
        Descriptor::Desc0A(v) => v,
        _ => unreachable!()
    };
    match &descriptors.next().unwrap() {
        Descriptor::Desc52(v) => v,
        _ => unreachable!()
    };
}
