use mpegts::psi::*;
use mpegts::{textcode, constants};

static DATA_09: &[u8] = &[0x09, 0x04, 0x09, 0x63, 0xe5, 0x01];
static DATA_0A: &[u8] = &[0x0A, 0x04, 0x65, 0x6e, 0x67, 0x01];
static DATA_0E: &[u8] = &[0x0e, 0x03, 0xc1, 0x2e, 0xbc];
static DATA_40: &[u8] = &[0x40, 0x06, 0x01, 0x43, 0x65, 0x73, 0x62, 0x6f];
static DATA_41: &[u8] = &[0x41, 0x06, 0x21, 0x85, 0x01, 0x21, 0x86, 0x01];
static DATA_43: &[u8] = &[0x43, 0x0b, 0x01, 0x23, 0x80, 0x00, 0x01, 0x30, 0xa1, 0x02, 0x75, 0x00, 0x03];
static DATA_44: &[u8] = &[0x44, 0x0b, 0x03, 0x46, 0x00, 0x00, 0xff, 0xf0, 0x05, 0x00, 0x68, 0x75, 0x00];
static DATA_4D: &[u8] = &[
    0x4d, 0x18, 0x72, 0x75, 0x73, 0x13, 0x01, 0xc1, 0xe2, 0xe0, 0xde, 0xd9, 0xda, 0xd0, 0x20, 0xdd,
    0xd0, 0x20, 0xb0, 0xdb, 0xef, 0xe1, 0xda, 0xd5, 0x2e, 0x00];
static DATA_4E: &[u8] = &[
    0x4e, 0x20, 0x00, 0x72, 0x75, 0x73, 0x00, 0x1a, 0x01, 0xb7, 0xd8, 0xdc, 0xd0, 0x20,
    0xd1, 0xeb, 0xe1, 0xe2, 0xe0, 0xde, 0x20, 0xdf, 0xe0, 0xd8, 0xd1, 0xdb, 0xd8, 0xd6, 0xd0, 0xd5,
    0xe2, 0xe1, 0xef, 0x2e];
static DATA_52: &[u8] = &[0x52, 0x01, 0x02];
static DATA_5A: &[u8] = &[0x5a, 0x0b, 0x02, 0xfa, 0xf0, 0x80, 0x1f, 0x81, 0x1a, 0xff, 0xff, 0xff, 0xff];

#[test]
fn test_09_parse() {
    let mut descriptors = Descriptors::default();
    descriptors.parse(DATA_09);

    let desc = match descriptors.iter().next().unwrap() {
        Descriptor::Desc09(v) => v,
        _ => unreachable!()
    };

    assert_eq!(desc.caid, 2403);
    assert_eq!(desc.pid, 1281);
    assert_eq!(desc.data, []);
}

#[test]
fn test_09_assemble() {
    let mut descriptors = Descriptors::default();
    descriptors.push(
        Descriptor::Desc09(
            Desc09 {
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
    let mut descriptors = Descriptors::default();
    descriptors.parse(DATA_0A);

    let desc = match descriptors.iter().next().unwrap() {
        Descriptor::Desc0A(v) => v,
        _ => unreachable!()
    };

    let item = &desc.items[0];
    assert_eq!(item.code, textcode::StringDVB::from_str("eng", 0));
    assert_eq!(item.audio_type, 1);
}

#[test]
fn test_0a_assemble() {
    let mut descriptors = Descriptors::default();
    descriptors.push(
        Descriptor::Desc0A(
            Desc0A {
                items: vec!(
                    Desc0A_Item {
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
    let mut descriptors = Descriptors::default();
    descriptors.parse(DATA_0E);

    let desc = match descriptors.iter().next().unwrap() {
        Descriptor::Desc0E(v) => v,
        _ => unreachable!()
    };

    assert_eq!(desc.bitrate, 77500);
}

#[test]
fn test_0e_assemble() {
    let mut descriptors = Descriptors::default();
    descriptors.push(
        Descriptor::Desc0E(
            Desc0E {
                bitrate: 77500
            }
        )
    );

    let mut assembled = Vec::new();
    descriptors.assemble(&mut assembled);

    assert_eq!(assembled.as_slice(), DATA_0E);
}

#[test]
fn test_40_parse() {
    let mut descriptors = Descriptors::default();
    descriptors.parse(DATA_40);

    let desc = match descriptors.iter().next().unwrap() {
        Descriptor::Desc40(v) => v,
        _ => unreachable!()
    };

    assert_eq!(desc.name, textcode::StringDVB::from_str("Cesbo", 5));
}

#[test]
fn test_40_assemble() {
    let mut descriptors = Descriptors::default();
    descriptors.push(
        Descriptor::Desc40(
            Desc40 {
                name: textcode::StringDVB::from_str("Cesbo", 5)
            }
        )
    );

    let mut assembled = Vec::new();
    descriptors.assemble(&mut assembled);

    assert_eq!(assembled.as_slice(), DATA_40);
}

#[test]
fn test_41_parse() {
    let mut descriptors = Descriptors::default();
    descriptors.parse(DATA_41);

    let desc = match descriptors.iter().next().unwrap() {
        Descriptor::Desc41(v) => v,
        _ => unreachable!()
    };

    let mut items = desc.items.iter();
    let item = items.next().unwrap();
    assert_eq!(item.0, 8581);
    assert_eq!(item.1, 1);
    let item = items.next().unwrap();
    assert_eq!(item.0, 8582);
    assert_eq!(item.1, 1);
}

#[test]
fn test_41_assemble() {
    let mut descriptors = Descriptors::default();
    descriptors.push(
        Descriptor::Desc41(
            Desc41 {
                items: vec![(8581, 1), (8582, 1)]
            }
        )
    );

    let mut assembled = Vec::new();
    descriptors.assemble(&mut assembled);

    assert_eq!(assembled.as_slice(), DATA_41);
}

#[test]
fn test_43_parse() {
    let mut descriptors = Descriptors::default();
    descriptors.parse(DATA_43);

    let desc = match descriptors.iter().next().unwrap() {
        Descriptor::Desc43(v) => v,
        _ => unreachable!()
    };

    assert_eq!(desc.frequency, 12380000);
    assert_eq!(desc.orbital_position, 780);
    assert_eq!(desc.west_east_flag, constants::POSITION_EAST);
    assert_eq!(desc.polarization, constants::POLARIZATION_V);
    assert_eq!(desc.rof, 0);
    assert_eq!(desc.s2, false);
    assert_eq!(desc.modulation, constants::MODULATION_DVB_S_QPSK);
    assert_eq!(desc.symbol_rate, 27500);
    assert_eq!(desc.fec, constants::FEC_3_4);
}

#[test]
fn test_43_assemble() {
    let mut descriptors = Descriptors::default();
    descriptors.push(
        Descriptor::Desc43(
            Desc43 {
                frequency: 12380000,
                orbital_position: 780,
                west_east_flag: constants::POSITION_EAST,
                polarization: constants::POLARIZATION_V,
                rof: 0,
                s2: false,
                modulation: constants::MODULATION_DVB_S_QPSK,
                symbol_rate: 27500,
                fec: constants::FEC_3_4
            }
        )
    );

    let mut assembled = Vec::new();
    descriptors.assemble(&mut assembled);

    assert_eq!(assembled.as_slice(), DATA_43);
}


#[test]
fn test_44_parse() {
    let mut descriptors = Descriptors::default();
    descriptors.parse(DATA_44);

    let desc = match descriptors.iter().next().unwrap() {
        Descriptor::Desc44(v) => v,
        _ => unreachable!()
    };

    assert_eq!(desc.frequency, 346000000);
    assert_eq!(desc.fec_outer, constants::FEC_OUTER_NOT_DEFINED);
    assert_eq!(desc.modulation, constants::MODULATION_DVB_C_256_QAM);
    assert_eq!(desc.symbol_rate, 6875);
    assert_eq!(desc.fec, constants::FEC_NOT_DEFINED);
}

#[test]
fn test_44_assemble() {
    let mut descriptors = Descriptors::default();
    descriptors.push(
        Descriptor::Desc44(
            Desc44 {
                frequency: 346000000,
                fec_outer: constants::FEC_OUTER_NOT_DEFINED,
                modulation: constants::MODULATION_DVB_C_256_QAM,
                symbol_rate: 6875,
                fec: constants::FEC_NOT_DEFINED
            }
        )
    );

    let mut assembled = Vec::new();
    descriptors.assemble(&mut assembled);

    assert_eq!(assembled.as_slice(), DATA_44);
}

#[test]
fn test_4d_parse() {
    let mut descriptors = Descriptors::default();
    descriptors.parse(DATA_4D);

    let desc = match descriptors.iter().next().unwrap() {
        Descriptor::Desc4D(v) => v,
        _ => unreachable!()
    };

    assert_eq!(desc.size(), DATA_4D.len());
    assert_eq!(desc.lang, textcode::StringDVB::from_str("rus", textcode::ISO6937));
    assert_eq!(desc.name, textcode::StringDVB::from_str("Стройка на Аляске.", textcode::ISO8859_5));
    assert!(desc.text.is_empty());
}

#[test]
fn test_4d_assemble() {
    let mut descriptors = Descriptors::default();
    descriptors.push(
        Descriptor::Desc4D(
            Desc4D {
                lang: textcode::StringDVB::from_str("rus", textcode::ISO6937),
                name: textcode::StringDVB::from_str("Стройка на Аляске.", textcode::ISO8859_5),
                text: textcode::StringDVB::default(),
            }
        )
    );

    let mut assembled = Vec::new();
    descriptors.assemble(&mut assembled);

    assert_eq!(assembled.as_slice(), DATA_4D);
}

#[test]
fn test_4e_parse() {
    let mut descriptors = Descriptors::default();
    descriptors.parse(DATA_4E);

    let desc = match descriptors.iter().next().unwrap() {
        Descriptor::Desc4E(v) => v,
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
    let mut descriptors = Descriptors::default();
    descriptors.push(
        Descriptor::Desc4E(
            Desc4E {
                number: 0,
                last_number: 0,
                lang: textcode::StringDVB::from_str("rus", textcode::ISO6937),
                items: Vec::new(),
                text: textcode::StringDVB::from_str("Зима быстро приближается.", textcode::ISO8859_5),
            }
        )
    );

    let mut assembled = Vec::new();
    descriptors.assemble(&mut assembled);

    assert_eq!(assembled.as_slice(), DATA_4E);
}

#[test]
fn test_52_parse() {
    let mut descriptors = Descriptors::default();
    descriptors.parse(DATA_52);

    let desc = match descriptors.iter().next().unwrap() {
        Descriptor::Desc52(v) => v,
        _ => unreachable!()
    };

    assert_eq!(desc.tag, 2);
}

#[test]
fn test_52_assemble() {
    let mut descriptors = Descriptors::default();
    descriptors.push(
        Descriptor::Desc52(
            Desc52 {
                tag: 2
            }
        )
    );

    let mut assembled = Vec::new();
    descriptors.assemble(&mut assembled);

    assert_eq!(assembled.as_slice(), DATA_52);
}


#[test]
fn test_5a_parse() {
    let mut descriptors = Descriptors::default();
    descriptors.parse(DATA_5A);

    let desc = match descriptors.iter().next().unwrap() {
        Descriptor::Desc5A(v) => v,
        _ => unreachable!()
    };

    assert_eq!(desc.frequency, 500000000);
    assert_eq!(desc.bandwidth, constants::BANDWIDTH_DVB_T_8MHZ);
    assert_eq!(desc.priority, true);
    assert_eq!(desc.time_slicing, true);
    assert_eq!(desc.mpe_fec, true);
    assert_eq!(desc.modulation, constants::MODULATION_DVB_T_64QAM);
    assert_eq!(desc.hierarchy, constants::HIERARCHY_DVB_T_NON_NATIVE);
    assert_eq!(desc.code_rate_hp, constants::CODE_RATE_DVB_T_2_3);
    assert_eq!(desc.code_rate_lp, 0);
    assert_eq!(desc.guard_interval, constants::GUARD_INTERVAL_1_4);
    assert_eq!(desc.transmission, constants::TRANSMISSION_MODE_8K);
    assert_eq!(desc.other_frequency_flag, false);
}

#[test]
fn test_5a_assemble() {
    let mut descriptors = Descriptors::default();
    descriptors.push(
        Descriptor::Desc5A(
            Desc5A {
                frequency: 500000000,
                bandwidth: constants::BANDWIDTH_DVB_T_8MHZ,
                priority: true,
                time_slicing: true,
                mpe_fec: true,
                modulation: constants::MODULATION_DVB_T_64QAM,
                hierarchy: constants::HIERARCHY_DVB_T_NON_NATIVE,
                code_rate_hp: constants::CODE_RATE_DVB_T_2_3,
                code_rate_lp: 0,
                guard_interval: constants::GUARD_INTERVAL_1_4,
                transmission: constants::TRANSMISSION_MODE_8K,
                other_frequency_flag: false
            }
        )
    );

    let mut assembled = Vec::new();
    descriptors.assemble(&mut assembled);

    assert_eq!(assembled.as_slice(), DATA_5A);
}
