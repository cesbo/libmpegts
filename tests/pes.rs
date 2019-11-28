use mpegts::es::pes;


const DATA: &[u8] = &[
    0x00, 0x00, 0x01, 0xe0, 0x42, 0xda, 0x80, 0xc0,
    0x0a, 0x31, 0x00, 0x0d, 0xdf, 0x89, 0x11, 0x00,
    0x0d, 0xc8, 0x13
];


#[test]
fn test_is_prefix() {
    assert_eq!(pes::is_prefix(DATA), true);
}


#[test]
fn test_get_pts() {
    assert_eq!(pes::is_syntax_spec(DATA), true);
    assert_eq!(pes::is_pts(DATA), true);
    assert_eq!(pes::get_pts(DATA), 225220);
}


#[test]
fn test_get_dts() {
    assert_eq!(pes::is_syntax_spec(DATA), true);
    assert_eq!(pes::is_dts(DATA), true);
    assert_eq!(pes::get_dts(DATA), 222217);
}
