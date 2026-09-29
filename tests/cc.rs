use libmpegts::ts::{CcChecker, CcStatus};
use libmpegts::ts::{PACKET_SIZE, TsPacketMut};

// Первый пакет на PID даёт First
#[test]
fn test_cc_first() {
    let mut checker = CcChecker::new();

    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 2);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::First);
}

// CC растёт на 1 в пакетах с payload (AFC 01)
#[test]
fn test_cc_payload_increment() {
    
}

// CC растёт на 1 в пакетах с AF и payload (AFC 11)
#[test]
fn test_cc_af_payload_increment() {}

// В пакетах только с AF (AFC 10) CC совпадает с предыдущим
#[test]
fn test_cc_af_only_same() {
    let mut checker = CcChecker::new();

    // Проверка AFC == 10 
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 2);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::First);

    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 2);
    packet.set_adaptation_field(2);

    assert_eq!(checker.check(&data), CcStatus::Ok);
}

// В пакете только с AF CC изменился - ошибка
#[test]
fn test_cc_af_only_changed() {}

// discontinuity_indicator разрешает скачок CC
#[test]
fn test_cc_discontinuity() {}

// Null-пакеты (PID 0x1FFF) не проверяются
#[test]
fn test_cc_null_pid() {}

// Переход 15 -> 0 штатный
#[test]
fn test_cc_wrap() {}

// Пропуск пакета даёт одну ошибку, дальше поток снова Ok
#[test]
fn test_cc_lost_packet() {}

// Два PID вперемешку считаются независимо
#[test]
fn test_cc_two_pids() {}

// После reset() следующий пакет снова First
#[test]
fn test_cc_reset() {}
