use mpegts::utils::crc32b;

#[test]
fn test_crc32b() {
    let s = "123456789";
    assert_eq!(crc32b(s), 0x0376e6e7);
}
