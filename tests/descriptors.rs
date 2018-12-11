extern crate mpegts;

use mpegts::psi;
use mpegts::textcode;

static DATA_09: &[u8] = &[0x09, 0x04, 0x09, 0x63, 0xe5, 0x01];
static DATA_0A: &[u8] = &[0x0A, 0x04, 0x65, 0x6e, 0x67, 0x01];
static DATA_0E: &[u8] = &[0x0e, 0x03, 0xc1, 0x2e, 0xbc];
static DATA_4D: &[u8] = &[
    0x4d, 0x18, 0x72, 0x75, 0x73, 0x13, 0x01, 0xc1, 0xe2, 0xe0, 0xde, 0xd9, 0xda, 0xd0, 0x20, 0xdd,
    0xd0, 0x20, 0xb0, 0xdb, 0xef, 0xe1, 0xda, 0xd5, 0x2e, 0x00];
static DATA_4E: &[u8] = &[
    0x4e, 0x20, 0x00, 0x72, 0x75, 0x73, 0x00, 0x1a, 0x01, 0xb7, 0xd8, 0xdc, 0xd0, 0x20,
    0xd1, 0xeb, 0xe1, 0xe2, 0xe0, 0xde, 0x20, 0xdf, 0xe0, 0xd8, 0xd1, 0xdb, 0xd8, 0xd6, 0xd0, 0xd5,
    0xe2, 0xe1, 0xef, 0x2e];
static DATA_52: &[u8] = &[0x52, 0x01, 0x02];

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


#[test]
fn test_0a_parse() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.parse(DATA_0A);

    let desc = match descriptors.iter().next().unwrap() {
        psi::Descriptor::Desc0A(v) => v,
        _ => unreachable!()
    };

    let item = &desc.items[0];
    assert_eq!(item.code, textcode::StringDVB::from_str("eng", 0));
    assert_eq!(item.audio_type, 1);
}

#[test]
fn test_0a_assemble() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.push(
        psi::Descriptor::Desc0A(
            psi::Desc0A {
                items: vec!(
                    psi::Desc0A_Item {
                        code: textcode::StringDVB::from_str("eng", 0),
                        audio_type: 1
                    }
                )
            }
        )
    );

    let mut assembled = Vec::new();
    descriptors.assemble(&mut assembled);

    assert_eq!(assembled.as_slice(), DATA_0A);
}


#[test]
fn test_0e_parse() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.parse(DATA_0E);

    let desc = match descriptors.iter().next().unwrap() {
        psi::Descriptor::Desc0E(v) => v,
        _ => unreachable!()
    };

    assert_eq!(desc.bitrate, 77500);
}

#[test]
fn test_0e_assemble() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.push(
        psi::Descriptor::Desc0E(
            psi::Desc0E {
                bitrate: 77500
            }
        )
    );

    let mut assembled = Vec::new();
    descriptors.assemble(&mut assembled);

    assert_eq!(assembled.as_slice(), DATA_0E);
}

#[test]
fn test_4d_parse() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.parse(DATA_4D);

    let desc = match descriptors.iter().next().unwrap() {
        psi::Descriptor::Desc4D(v) => v,
        _ => unreachable!()
    };

    assert_eq!(desc.size(), DATA_4D.len());
    assert_eq!(desc.lang, textcode::StringDVB::from_str("rus", textcode::ISO6937));
    assert_eq!(desc.name, textcode::StringDVB::from_str("Стройка на Аляске.", textcode::ISO8859_5));
    assert!(desc.text.is_empty());
}

#[test]
fn test_4d_assemble() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.push(
        psi::Descriptor::Desc4D(
            psi::Desc4D {
                lang: textcode::StringDVB::from_str("rus", textcode::ISO6937),
                name: textcode::StringDVB::from_str("Стройка на Аляске.", textcode::ISO8859_5),
                text: textcode::StringDVB::from_str("", textcode::ISO8859_5),
            }
        )
    );

    assert_eq!(descriptors.size(), DATA_4D.len());

    let mut assembled = Vec::new();
    descriptors.assemble(&mut assembled);

    assert_eq!(assembled.as_slice(), DATA_4D);
}

#[test]
fn test_4e_parse() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.parse(DATA_4E);

    let desc = match descriptors.iter().next().unwrap() {
        psi::Descriptor::Desc4E(v) => v,
        _ => unreachable!()
    };

    assert_eq!(desc.size(), DATA_4E.len());
    assert_eq!(desc.number, 0);
    assert_eq!(desc.last_number, 0);
    assert_eq!(desc.lang, textcode::StringDVB::from_str("rus", textcode::ISO6937));
    assert_eq!(desc.text, textcode::StringDVB::from_str("Зима быстро приближается.", textcode::ISO8859_5));
}

#[test]
fn test_4e_assemble() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.push(
        psi::Descriptor::Desc4E(
            psi::Desc4E {
                number: 0,
                last_number: 0,
                lang: textcode::StringDVB::from_str("rus", textcode::ISO6937),
                items: Vec::new(),
                text: textcode::StringDVB::from_str("Зима быстро приближается.", textcode::ISO8859_5),
            }
        )
    );

    assert_eq!(descriptors.size(), DATA_4E.len());

    let mut assembled = Vec::new();
    descriptors.assemble(&mut assembled);

    assert_eq!(assembled.as_slice(), DATA_4E);
}

#[test]
fn test_52_parse() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.parse(DATA_52);

    let desc = match descriptors.iter().next().unwrap() {
        psi::Descriptor::Desc52(v) => v,
        _ => unreachable!()
    };

    assert_eq!(desc.tag, 2);
}

#[test]
fn test_52_assemble() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.push(
        psi::Descriptor::Desc52(
            psi::Desc52 {
                tag: 2
            }
        )
    );

    let mut assembled = Vec::new();
    descriptors.assemble(&mut assembled);

    assert_eq!(assembled.as_slice(), DATA_52);
}
