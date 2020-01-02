use bitwrap::BitWrap;
use mpegts::psi::*;
mod data;

#[test]
fn test_parse_tdt() {
    let mut psi = Psi::default();
    psi.mux(data::TDT);

    let mut tdt = Tdt::default();
    tdt.unpack(&psi.buffer).unwrap();

    assert_eq!(tdt.time, 1547057412);
}

#[test]
fn test_assemble_tdt() {
    let mut tdt = Tdt::default();
    tdt.time = 1547057412;

    let mut buffer: [u8; 1024] = [0; 1024];
    let result = tdt.pack(&mut buffer).unwrap();
    assert_eq!(&buffer[.. result - 4], &data::TDT[5 .. result + 5 - 4]);
}
