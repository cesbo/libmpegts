use mpegts::psi::{
    MjdFrom,
    MjdTo,
};

#[test]
fn test_from_mjd() {
    assert_eq!(u64::from_mjd([0xc0, 0x79]), 750470400);
}

#[test]
fn test_into_mjd() {
    assert_eq!(750470400u64.into_mjd(), 0xc079);
}
