use mpegts::set_bits;
use mpegts::constants::*;

struct Sat {
    west_east_flag: u8,
    polarization: u8,
    rof: u8,
    s2: u8,
    modulation: u8,
}

#[test]
fn test_set_bits() {
    let x = Sat {
        west_east_flag: POSITION_EAST,
        polarization: POLARIZATION_V,
        rof: ROF_A035,
        s2: 1,
        modulation: MODULATION_DVB_S_8PSK,
    };

    let b1: u8 =
        (x.west_east_flag << 7) |
        (x.polarization << 5) |
        (x.rof << 3) |
        (x.s2 << 2) |
        x.modulation;

    let b2 = set_bits!(8,
        x.west_east_flag, 1,
        x.polarization, 2,
        x.rof, 2,
        x.s2, 1,
        x.modulation, 2
    );

    assert_eq!(b1, b2);
}
