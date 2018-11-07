extern crate mpegts;

mod data;
use mpegts::psi::{Psi, Pmt};


#[test]
fn test_parse_pmt() {
    let mut psi = Psi::default();
    psi.mux(&data::PMT);
    assert!(psi.check());

    let mut pmt = Pmt::default();
    pmt.parse(&psi);
}
