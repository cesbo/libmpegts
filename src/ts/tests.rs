use crate::ts::{
    self,
    TS
};


#[test]
fn test_get_payload_offset() {
    let mut packet: Vec<u8> = vec![0x47, 0x40, 0x11, 0x10, 0x00];
    let ts = TS::new(&mut packet);
    assert!(ts.is_payload());
    assert_eq!(ts.get_payload_offset(), 4);

    let mut packet: Vec<u8> = vec![0x47, 0x40, 0x2d, 0xf0, 0x19, 0x00];
    let ts = TS::new(&mut packet);
    assert!(ts.is_payload());
    assert_eq!(ts.get_payload_offset(), 4 + 1 + 0x19);
}


#[test]
fn test_set_payload_1() {
    let mut packet = vec![0x47, 0x00, 0x00, 0x00];
    let mut ts = TS::new(&mut packet);
    ts.set_payload_1();
    assert_eq!(packet[3], 0x10);
}


#[test]
fn test_set_payload_0() {
    let mut packet = vec![0x47, 0x00, 0x00, 0xFF];
    let mut ts = TS::new(&mut packet);
    ts.set_payload_0();
    assert_eq!(packet[3], 0xEF);
}


#[test]
fn test_set_pusi_1() {
    let mut packet = vec![0x47, 0x00];
    let mut ts = TS::new(&mut packet);
    ts.set_pusi_1();
    assert_eq!(packet[1], 0x40);
}


#[test]
fn test_set_pusi_0() {
    let mut packet = vec![0x47, 0xFF];
    let mut ts = TS::new(&mut packet);
    ts.set_pusi_0();
    assert_eq!(packet[1], 0xBF);
}


#[test]
fn test_set_pid() {
    let mut packet = vec![0x47, 0x00, 0x00];
    let mut ts = TS::new(&mut packet);
    ts.set_pid(8191);
    assert_eq!(&[0x1F, 0xFF], &packet[1..]);
}


#[test]
fn test_is_pcr() {
    let mut packet: Vec<u8> = vec![0x47, 0x01, 0x00, 0x20, 0xb7, 0x10];
    let ts = TS::new(&mut packet);
    assert!(ts.is_pcr());

    let mut packet: Vec<u8> = vec![0x47, 0x40, 0x11, 0x10, 0x00];
    let ts = TS::new(&mut packet);
    assert!(!ts.is_pcr());
}


#[test]
fn test_pcr_delta() {
    let current_pcr = 20000;
    let last_pcr = current_pcr - 10000;
    assert_eq!(ts::pcr_delta(last_pcr, current_pcr), 10000);
}


#[test]
fn test_pcr_delta_overflow() {
    let current_pcr = 5000;
    let last_pcr = ts::PCR_MAX - 5000;
    assert_eq!(ts::pcr_delta(last_pcr, current_pcr), 10000);
}


#[test]
fn test_get_pcr() {
    let mut packet: Vec<u8> = vec![
        0x47, 0x01, 0x00, 0x20, 0xB7, 0x10, 0x00, 0x02, 0x32, 0x89, 0x7E, 0xF7
    ];
    
    let ts = TS::new(&mut packet);
    assert!(ts.is_pcr());
    assert_eq!(ts.get_pcr(), 86405647);
}


#[test]
fn test_set_pcr() {
    let mut packet: Vec<u8> = vec![
        0x47, 0x01, 0x00, 0x20, 0xB7, 0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
    ];
    
    let mut ts = TS::new(&mut packet);
    ts.set_pcr(86405647);
    assert_eq!(&[0x00, 0x02, 0x32, 0x89, 0x7E, 0xF7], &packet[6 ..]);
}
