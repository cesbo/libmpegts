use libmpegts::utils::{
    MjdFrom,
    MjdTo,
};

#[test]
fn test_from_mjd() {
    assert_eq!(u64::from_mjd([0xc0, 0x79]), 750470400);
}

#[test]
fn test_from_mjd_before_1970() {
    assert_eq!(u64::from_mjd([0x9e, 0x8a]), 0); // MJD 40586, 1969-12-31
    assert_eq!(u64::from_mjd([0x00, 0x00]), 0);
}

#[test]
fn test_into_mjd() {
    assert_eq!(750470400u64.into_mjd(), [0xc0, 0x79]);
}
