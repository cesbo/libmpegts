use libmpegts::utils::textcode::{
    Charset,
    DvbTextRef,
};

#[test]
fn test_decode_iso6937() {
    let e = "Hello!".as_bytes();
    let x = DvbTextRef::try_from(e).expect("expected string");
    assert_eq!(x.charset(), Charset::Iso6937);
    assert_eq!(x.to_string(), String::from_utf8_lossy(e));
}

#[test]
fn test_decode_iso8859() {
    let e: &[u8] = &[0x10, 0x00, 0x05, 0xbf, 0xe0, 0xd8, 0xd2, 0xd5, 0xe2, 0x21];
    let x = DvbTextRef::try_from(e).expect("expected string");
    assert_eq!(x.charset(), Charset::Iso8859_5);
    assert_eq!(&x.to_string(), "Привет!");
}

#[test]
fn test_decode_geo() {
    let e: &[u8] = &[
        0x1E, 0xE1, 0xD0, 0xE5, 0xD0, 0xE0, 0xD7, 0xD5, 0xD4, 0xDA, 0xDD,
    ];
    let x = DvbTextRef::try_from(e).expect("expected string");
    assert_eq!(x.charset(), Charset::Geo);
    assert_eq!(&x.to_string(), "საქართველო");
}
