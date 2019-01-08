use mpegts::mjd::MJD;

#[test]
fn test_from_mjd() {
    assert_eq!(0xc079u16.from_mjd(), 750470400);
}

#[test]
fn test_to_mjd() {
    assert_eq!(u16::to_mjd(750470400), 0xc079);
}
