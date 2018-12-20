extern crate mpegts;

use mpegts::{psi, textcode, constants};


static DATA_09: &[u8] = &[0x09, 0x04, 0x09, 0x63, 0xe5, 0x01];
static DATA_0A: &[u8] = &[0x0A, 0x04, 0x65, 0x6e, 0x67, 0x01];
static DATA_0E: &[u8] = &[0x0e, 0x03, 0xc1, 0x2e, 0xbc];
static DATA_40: &[u8] = &[0x40, 0x06, 0x01, 0x43, 0x65, 0x73, 0x62, 0x6f];
static DATA_43: &[u8] = &[0x43, 0x0b, 0x01, 0x23, 0x80, 0x00, 0x01, 0x30, 0xa1, 0x02, 0x75, 0x00, 0x03];
static DATA_44: &[u8] = &[0x44, 0x0b, 0x03, 0x46, 0x00, 0x00, 0xff, 0xf0, 0x05, 0x00, 0x68, 0x75, 0x00];
static DATA_52: &[u8] = &[0x52, 0x01, 0x02];
static DATA_5A: &[u8] = &[0x5a, 0x0b, 0x02, 0xfa, 0xf0, 0x80, 0x1f, 0x81, 0x1a, 0xff, 0xff, 0xff, 0xff];


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
fn test_40_parse() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.parse(DATA_40);

    let desc = match descriptors.iter().next().unwrap() {
        psi::Descriptor::Desc40(v) => v,
        _ => unreachable!()
    };

    assert_eq!(desc.name, textcode::StringDVB::from_str("Cesbo", 5));
}

#[test]
fn test_40_assemble() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.push(
        psi::Descriptor::Desc40(
            psi::Desc40 {
                name: textcode::StringDVB::from_str("Cesbo", 5)
            }
        )
    );

    let mut assembled = Vec::new();
    descriptors.assemble(&mut assembled);

    assert_eq!(assembled.as_slice(), DATA_40);
}


#[test]
fn test_43_parse() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.parse(DATA_43);

    let desc = match descriptors.iter().next().unwrap() {
        psi::Descriptor::Desc43(v) => v,
        _ => unreachable!()
    };

    assert_eq!(desc.frequency, 12380000);
    assert_eq!(desc.orbital_position, 780);
    assert_eq!(desc.west_east_flag, constants::SIDE_EAST);
    assert_eq!(desc.polarization, constants::POLARIZATION_VERTICAL);
    assert_eq!(desc.rof, 0);
    assert_eq!(desc.s2, false);
    assert_eq!(desc.modulation, constants::MODULATION_DVB_S_QPSK);
    assert_eq!(desc.symbol_rate, 27500);
    assert_eq!(desc.fec, constants::FEC_3_4);
}

#[test]
fn test_43_assemble() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.push(
        psi::Descriptor::Desc43(
            psi::Desc43 {
                frequency: 12380000,
                orbital_position: 780,
                west_east_flag: constants::SIDE_EAST,
                polarization: constants::POLARIZATION_VERTICAL,
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
    let mut descriptors = psi::Descriptors::default();
    descriptors.parse(DATA_44);

    let desc = match descriptors.iter().next().unwrap() {
        psi::Descriptor::Desc44(v) => v,
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
    let mut descriptors = psi::Descriptors::default();
    descriptors.push(
        psi::Descriptor::Desc44(
            psi::Desc44 {
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


#[test]
fn test_5a_parse() {
    let mut descriptors = psi::Descriptors::default();
    descriptors.parse(DATA_5A);

    let desc = match descriptors.iter().next().unwrap() {
        psi::Descriptor::Desc5A(v) => v,
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
    let mut descriptors = psi::Descriptors::default();
    descriptors.push(
        psi::Descriptor::Desc5A(
            psi::Desc5A {
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
