extern crate mpegts;

mod data;
use mpegts::psi;


#[test]
fn test_parse_nit() {
    let mut psi = psi::Psi::default();
    psi.mux(&data::NIT_DVBS);
    assert!(psi.check());

    let mut nit = psi::Nit::default();
    nit.parse(&psi);

    println!("{:?}", nit);
}
