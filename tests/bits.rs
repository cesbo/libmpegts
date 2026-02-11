use libmpegts::{
    constants::*,
    pack_bits,
};

struct Sat {
    west_east_flag: u8,
    polarization: u8,
    rof: u8,
    s2: u8,
    modulation: u8,
}

#[test]
fn test_pack_bits_u8() {
    let x = Sat {
        west_east_flag: POSITION_EAST,
        polarization: POLARIZATION_V,
        rof: ROF_A035,
        s2: 1,
        modulation: MODULATION_DVB_S_8PSK,
    };

    let expected: u8 =
        (x.west_east_flag << 7) | (x.polarization << 5) | (x.rof << 3) | (x.s2 << 2) | x.modulation;

    let result = pack_bits!(
        u8,
        west_east_flag: 1 => x.west_east_flag,
        polarization: 2 => x.polarization,
        rof: 2 => x.rof,
        s2: 1 => x.s2,
        modulation: 2 => x.modulation
    );

    assert_eq!([expected], result);
}

#[test]
fn test_pack_bits_u16() {
    assert_eq!(
        pack_bits!(u16,
            field_a: 4 => 0xA,
            field_b: 12 => 0xBCD,
        ),
        [0xAB, 0xCD]
    );

    assert_eq!(
        pack_bits!(u16,
            field_a: 4 => 0xFF, // 0xFF overflows
            field_b: 12 => 0x000,
        ),
        [0xF0, 0x00]
    );
}

#[test]
fn test_pack_bits_psi_version() {
    let expected = 0xC0 | ((0b10101 << 1) & 0x3E) | 0x01;
    let result = pack_bits!(u8,
        reserved: 2 => 0b11,
        version: 5 => 0b10101,
        current_next_indicator: 1 => 1
    );
    assert_eq!([expected], result);
}
