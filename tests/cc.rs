use libmpegts::ts::{CcChecker, CcStatus};
use libmpegts::ts::{PACKET_SIZE, PID_NULL, TsPacketMut};

// AFC 01: только payload
fn payload_packet(pid: u16, cc: u8) -> [u8; PACKET_SIZE] {
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(pid, cc);
    packet.set_payload();
    data
}

// AFC 10: только AF
fn af_only_packet(pid: u16, cc: u8) -> [u8; PACKET_SIZE] {
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(pid, cc);
    packet.set_adaptation_field(2);
    data
}

// AFC 11: AF + payload
fn af_payload_packet(pid: u16, cc: u8) -> [u8; PACKET_SIZE] {
    let mut data = af_only_packet(pid, cc);
    TsPacketMut::from(&mut data).set_payload();
    data
}

// AFC 11 + discontinuity_indicator
fn di_packet(pid: u16, cc: u8) -> [u8; PACKET_SIZE] {
    let mut data = af_payload_packet(pid, cc);
    TsPacketMut::from(&mut data).set_discontinuity();
    data
}

// Первый пакет на PID даёт First
#[test]
fn test_cc_first() {
    let mut checker = CcChecker::new();

    assert_eq!(checker.check(&payload_packet(256, 2)), CcStatus::First);
}

// CC растёт на 1 в пакетах с payload (AFC 01)
#[test]
fn test_cc_payload_increment() {
    let mut checker = CcChecker::new();

    assert_eq!(checker.check(&payload_packet(256, 2)), CcStatus::First);
    // Следующий пакет с тем же PID и payload, CC должен увеличиться на 1
    assert_eq!(checker.check(&payload_packet(256, 3)), CcStatus::Ok);
}

// CC растёт на 1 в пакетах с AF и payload (AFC 11)
#[test]
fn test_cc_af_payload_increment() {
    let mut checker = CcChecker::new();

    assert_eq!(checker.check(&af_payload_packet(256, 2)), CcStatus::First);
    assert_eq!(checker.check(&af_payload_packet(256, 3)), CcStatus::Ok);
}

// В пакетах только с AF (AFC 10) CC совпадает с предыдущим
#[test]
fn test_cc_af_only_same() {
    let mut checker = CcChecker::new();

    assert_eq!(checker.check(&payload_packet(256, 2)), CcStatus::First);
    // Проверка AFC == 10
    assert_eq!(checker.check(&af_only_packet(256, 2)), CcStatus::Ok);
}

// В пакете только с AF CC изменился - ошибка
#[test]
fn test_cc_af_only_changed() {
    let mut checker = CcChecker::new();

    assert_eq!(checker.check(&payload_packet(256, 2)), CcStatus::First);
    // Следующий пакет только с AF, CC изменился - должно быть Error
    assert_eq!(checker.check(&af_only_packet(256, 3)), CcStatus::Error { expected: 2, got: 3 });
}

// discontinuity_indicator разрешает скачок CC
#[test]
fn test_cc_discontinuity() {
    let mut checker = CcChecker::new();

    assert_eq!(checker.check(&payload_packet(256, 2)), CcStatus::First);
    // Пакет AF + payload с discontinuity_indicator, CC скачет 2 -> 9
    assert_eq!(checker.check(&di_packet(256, 9)), CcStatus::Discontinuity);
    // После скачка отсчёт идёт от нового CC
    assert_eq!(checker.check(&payload_packet(256, 10)), CcStatus::Ok);
}

// Null-пакеты (PID_NULL) не проверяются
#[test]
fn test_cc_null_pid() {
    let mut checker = CcChecker::new();

    assert_eq!(checker.check(&payload_packet(PID_NULL, 2)), CcStatus::Ok);
    // Проверка должна игнорировать null-пакеты, поэтому скачок CC тоже даёт Ok
    assert_eq!(checker.check(&payload_packet(PID_NULL, 9)), CcStatus::Ok);
}

// Переход 15 -> 0 штатный
#[test]
fn test_cc_wrap() {
    let mut checker = CcChecker::new();

    assert_eq!(checker.check(&payload_packet(256, 15)), CcStatus::First);
    assert_eq!(checker.check(&payload_packet(256, 0)), CcStatus::Ok);
}

// Пропуск пакета даёт одну ошибку, дальше поток снова Ok
#[test]
fn test_cc_lost_packet() {
    let mut checker = CcChecker::new();

    assert_eq!(checker.check(&payload_packet(256, 2)), CcStatus::First);
    // Пропущен пакет с CC = 3
    assert_eq!(checker.check(&payload_packet(256, 4)), CcStatus::Error { expected: 3, got: 4 });
    assert_eq!(checker.check(&payload_packet(256, 5)), CcStatus::Ok);
}

// Два PID вперемешку считаются независимо
#[test]
fn test_cc_two_pids() {
    let mut checker = CcChecker::new();

    assert_eq!(checker.check(&payload_packet(256, 1)), CcStatus::First);
    assert_eq!(checker.check(&payload_packet(257, 5)), CcStatus::First);
    assert_eq!(checker.check(&payload_packet(256, 2)), CcStatus::Ok);
    assert_eq!(checker.check(&payload_packet(257, 6)), CcStatus::Ok);
    // PID 256: ожидался CC = 3
    assert_eq!(checker.check(&payload_packet(256, 4)), CcStatus::Error { expected: 3, got: 4 });
    // Ошибка на PID 256 не влияет на PID 257
    assert_eq!(checker.check(&payload_packet(257, 7)), CcStatus::Ok);
    // Счётчик PID 256 подстроился под CC = 4
    assert_eq!(checker.check(&payload_packet(256, 5)), CcStatus::Ok);
}

// После reset() следующий пакет снова First
#[test]
fn test_cc_reset() {
    let mut checker = CcChecker::new();

    assert_eq!(checker.check(&payload_packet(256, 1)), CcStatus::First);

    checker.reset();

    // CC = 2 без сброса дал бы Ok, после сброса пакет снова первый
    assert_eq!(checker.check(&payload_packet(256, 2)), CcStatus::First);
}
