use mpegts::psi::{
    MJDFrom,
    MJDTo,
};

#[test]
fn test_from_mjd() {
    assert_eq!(u16::from_mjd(0xc079), 750470400);
}

#[test]
fn test_into_mjd() {
    assert_eq!(750470400u64.into_mjd(), 0xc079);
}
