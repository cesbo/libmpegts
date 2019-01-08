use mpegts::bcd::{BCD, BCDTime};

#[test]
fn test_from_bcd_u8() {
    assert_eq!(u8::from_bcd(0x12), 12);
}

#[test]
fn test_to_bcd_u8() {
    assert_eq!(0x12, 12u8.to_bcd());
}

#[test]
fn test_from_bcd_u16() {
    assert_eq!(u16::from_bcd(0x1234), 1234);
}

#[test]
fn test_to_bcd_u16() {
    assert_eq!(0x12, 12u8.to_bcd());
    assert_eq!(0x1234, 1234u16.to_bcd());
}

#[test]
fn test_from_bcd_u32() {
    assert_eq!(u32::from_bcd(0x12345678), 12345678);
}

#[test]
fn test_to_bcd_u32() {
    assert_eq!(0x12345678, 12345678u32.to_bcd());
}

#[test]
fn test_from_bcd_time() {
    assert_eq!(0x014530u32.from_bcd_time(), 1 * 3600 + 45 * 60 + 30);
}

#[test]
fn test_to_bcd_time() {
    assert_eq!(0x014530, ((1 * 3600 + 45 * 60 + 30) as u32).to_bcd_time());
}