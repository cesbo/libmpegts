extern crate mpegts;
use mpegts::psi;


static DATA_09: &[u8] = &[0x09, 0x04, 0x09, 0x63, 0xe5, 0x01];

#[test]
fn test_09_parse() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.parse(DATA_09);

    let desc = match descriptors.iter().next().unwrap() {
        psi::Descriptor::Desc09(v) => v,
        _ => unreachable!()
    };

    assert_eq!(desc.caid, 2403);
    assert_eq!(desc.pid, 1281);
    assert_eq!(desc.data, []);
}

#[test]
fn test_09_assemble() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.push(
        psi::Descriptor::Desc09(
            psi::Desc09 {
                caid: 2403,
                pid: 1281,
                data: Vec::new()
            }
        )
    );

    let mut assembled = Vec::new();
    descriptors.assemble(&mut assembled);

    assert_eq!(assembled.as_slice(), DATA_09);
}
