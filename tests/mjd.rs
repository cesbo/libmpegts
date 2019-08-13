use mpegts::psi::{
    MJDFrom,
    MJDTo,
};

#[test]
fn test_from_mjd() {
    assert_eq!(0xc079u16.from_mjd(), 750470400);
}

#[test]
fn test_to_mjd() {
    assert_eq!(750470400u64.to_mjd(), 0xc079);
}
