extern crate mpegts;
use mpegts::ts::*;

#[test]
fn test_set_payload_1() {
    let mut packet = new_ts();
    set_payload_1(&mut packet);
    assert_eq!(packet[3], 0x10);
}

#[test]
fn test_set_payload_0() {
    let mut packet = new_ts();
    packet[3] = 0xFF;
    set_payload_0(&mut packet);
    assert_eq!(packet[3], 0xEF);
}

#[test]
fn test_set_pusi_1() {
    let mut packet = new_ts();
    set_pusi_1(&mut packet);
    assert_eq!(packet[1], 0x40);
}

#[test]
fn test_set_pusi_0() {
    let mut packet = new_ts();
    packet[1] = 0xFF;
    set_pusi_0(&mut packet);
    assert_eq!(packet[1], 0xBF);
}

#[test]
fn test_set_pid() {
    let mut packet = new_ts();
    set_pusi_1(&mut packet);
    set_pid(&mut packet, 8191);
    assert_eq!(packet[1], 0x5F);
    assert_eq!(packet[2], 0xFF);
}
