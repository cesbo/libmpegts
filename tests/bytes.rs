use mpegts::base::Bytes;

#[test]
fn test_get_bytes_u16() {
    let data: &[u8] = &[0x12, 0x34];
    assert_eq!(data[0 ..].get_u16(), 0x1234);
}

#[test]
fn test_set_bytes_u16() {
    let mut data = Vec::<u8>::new();
    data.resize(2, 0x00);
    data[0 ..].set_u16(0x1234);
    assert_eq!(data[0], 0x12);
    assert_eq!(data[1], 0x34);
}

#[test]
fn test_get_bytes_u24() {
    let data: &[u8] = &[0x12, 0x34, 0xAB];
    assert_eq!(data[0 ..].get_u24(), 0x1234AB);
}

#[test]
fn test_set_bytes_u24() {
    let mut data = Vec::<u8>::new();
    data.resize(3, 0x00);
    data[0 ..].set_u24(0x1234AB);
    assert_eq!(data[0], 0x12);
    assert_eq!(data[1], 0x34);
    assert_eq!(data[2], 0xAB);
}

#[test]
fn test_get_bytes_u32() {
    let data: &[u8] = &[0x12, 0x34, 0xAB, 0xCD];
    assert_eq!(data[0 ..].get_u32(), 0x1234ABCD);
}

#[test]
fn test_set_bytes_u32() {
    let mut data = Vec::<u8>::new();
    data.resize(4, 0x00);
    data[0 ..].set_u32(0x1234ABCD);
    assert_eq!(data[0], 0x12);
    assert_eq!(data[1], 0x34);
    assert_eq!(data[2], 0xAB);
    assert_eq!(data[3], 0xCD);
}
