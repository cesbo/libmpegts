extern crate mpegts;

use mpegts::psi;
use mpegts::textcode;


static DATA_09: &[u8] = &[0x09, 0x04, 0x09, 0x63, 0xe5, 0x01];
static DATA_0A: &[u8] = &[0x0A, 0x04, 0x65, 0x6e, 0x67, 0x01];


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

    let lang = &desc.languages[0];
    assert_eq!(lang.code, textcode::StringDVB::from_str("eng", 0));
    assert_eq!(lang.audio_type, 1);
}

#[test]
fn test_0a_assemble() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.push(
        psi::Descriptor::Desc0A(
            psi::Desc0A {
                languages: vec!(
                    psi::Language {
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
