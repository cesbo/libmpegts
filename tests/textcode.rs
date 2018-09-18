extern crate mpegts;
use mpegts::textcode::*;

#[test]
fn test_encode_iso6937() {
    let e = "Hello!";
    let x = StringDVB::from_str(e, ISO6937);
    assert_eq!(x.as_bytes(), e.as_bytes());
    assert_eq!(x.to_string(), e);
}

#[test]
fn test_encode_iso8859() {
    let e = "Привет!";
    let x = StringDVB::from_str(e, ISO8859_5);
    let t: &[u8] = &[0xbf, 0xe0, 0xd8, 0xd2, 0xd5, 0xe2, 0x21];
    assert_eq!(x.as_bytes(), t);
}

#[test]
fn test_encode_utf8() {
    let e = "Привет!";
    let x = StringDVB::from_str(e, UTF8);
    assert_eq!(x.as_bytes(), e.as_bytes());
}

#[test]
fn test_decode_iso6937() {
    let e = b"Hello!";
    let x = StringDVB::from_raw(e);
    assert_eq!(x.get_codepage(), ISO6937);
    assert_eq!(x.to_string(), String::from_utf8_lossy(e));
}

#[test]
fn test_decode_iso8859() {
    let e: &[u8] = &[0x10, 0x00, 0x05, 0xbf, 0xe0, 0xd8, 0xd2, 0xd5, 0xe2, 0x21];
    let x = StringDVB::from_raw(e);
    assert_eq!(x.get_codepage(), ISO8859_5);
    assert_eq!(&x.to_string(), "Привет!");
}
