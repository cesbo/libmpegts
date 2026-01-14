use mpegts::utils::{
    Bcd,
    BcdTime,
};

#[test]
fn test_from_bcd_u8() {
    assert_eq!(u8::from_bcd(0x12), 12);
}

#[test]
fn test_into_bcd_u8() {
    assert_eq!(0x12, 12u8.into_bcd());
}

#[test]
fn test_from_bcd_u16() {
    assert_eq!(u16::from_bcd([0x12, 0x34]), 1234);
}

#[test]
fn test_into_bcd_u16() {
    assert_eq!(0x12, 12u8.into_bcd());
    assert_eq!([0x12, 0x34], 1234u16.into_bcd());
}

#[test]
fn test_from_bcd_u32() {
    assert_eq!(u32::from_bcd([0x12, 0x34, 0x56, 0x78]), 12345678);
}

#[test]
fn test_into_bcd_u32() {
    assert_eq!([0x12, 0x34, 0x56, 0x78], 12345678u32.into_bcd());
}

#[test]
fn test_u32_from_bcd_time() {
    assert_eq!(
        u32::from_bcd_time([0x01, 0x45, 0x30]),
        1 * 3600 + 45 * 60 + 30
    );
}

#[test]
fn test_u32_into_bcd_time() {
    assert_eq!(
        [0x01, 0x45, 0x30],
        ((1 * 3600 + 45 * 60 + 30) as u32).into_bcd_time()
    );
}

#[test]
fn test_u16_from_bcd_time() {
    assert_eq!(u16::from_bcd_time([0x01, 0x45]), 1 * 60 + 45);
}

#[test]
fn test_u16_into_bcd_time() {
    assert_eq!([0x00, 0x01, 0x45], ((1 * 60 + 45) as u32).into_bcd_time());
}

#[test]
fn test_005959_into_bcd_time() {
    assert_eq!([0x00, 0x59, 0x59], 1332637199u32.into_bcd_time());
}
