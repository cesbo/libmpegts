use bitwrap::BitWrap;
use mpegts::psi::*;
mod data;


#[test]
fn test_parse_tot() {
    let mut psi = Psi::default();
    psi.mux(data::TOT);

    let mut tot = Tot::default();
    tot.unpack(&psi.buffer).unwrap();

    assert_eq!(tot.time, 1547057412);
}


#[test]
fn test_assemble_tot() {
    let mut tot = Tot::default();
    tot.time = 1547057412;
    tot.descriptors.push(Descriptor::DescRaw(
        vec![0x9a, 0x0a, 0xe4, 0xb8, 0x02, 0x00, 0x00, 0xe5, 0xa6, 0x02, 0x00, 0x00],
    ));

    let mut buffer: [u8; 1024] = [0; 1024];
    let result = tot.pack(&mut buffer).unwrap();
    assert_eq!(&buffer[.. result], &data::TOT[5 .. result + 5]);
}
