use libmpegts::ts::{CcChecker, CcStatus};
use libmpegts::ts::{PACKET_SIZE, PID_NULL, TsPacketMut};

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
    let mut checker = CcChecker::new();

    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 2);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::First);

    // Следующий пакет с тем же PID и payload, CC должен увеличиться на 1
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 3);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::Ok);
}

// CC растёт на 1 в пакетах с AF и payload (AFC 11)
#[test]
fn test_cc_af_payload_increment() {
    let mut checker = CcChecker::new();

    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 2);
    packet.set_adaptation_field(2);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::First);

    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 3);
    packet.set_adaptation_field(2);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::Ok);
}

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
fn test_cc_af_only_changed() {
    let mut checker = CcChecker::new();

    // Первый пакет с payload
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 2);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::First);

    // Следующий пакет только с AF, CC изменился - должно быть Error
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 3);
    packet.set_adaptation_field(2);

    assert_eq!(checker.check(&data), CcStatus::Error { expected: 2, got: 3 });
}

// discontinuity_indicator разрешает скачок CC
#[test]
fn test_cc_discontinuity() {
    let mut checker = CcChecker::new();

    // Первый пакет с payload
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 2);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::First);

    // Пакет AF + payload с discontinuity_indicator, CC скачет 2 -> 9
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 9);
    packet.set_adaptation_field(2);
    packet.set_payload();
    packet.set_discontinuity();

    assert_eq!(checker.check(&data), CcStatus::Discontinuity);

    // После скачка отсчёт идёт от нового CC
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 10);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::Ok);
}

// Null-пакеты (PID_NULL) не проверяются
#[test]
fn test_cc_null_pid() {
    let mut checker = CcChecker::new();

    // Null-пакет с payload
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(PID_NULL, 2);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::Ok);

    // Следующий null-пакет с изменившимся CC
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(PID_NULL, 9);
    packet.set_payload();

    // Проверка должна игнорировать null-пакеты, поэтому статус остаётся Ok
    assert_eq!(checker.check(&data), CcStatus::Ok);
}

// Переход 15 -> 0 штатный
#[test]
fn test_cc_wrap() {
    let mut checker = CcChecker::new();

    // Первый пакет с payload и CC = 15
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 15);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::First);

    // Следующий пакет с payload и CC = 0
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 0);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::Ok);
}

// Пропуск пакета даёт одну ошибку, дальше поток снова Ok
#[test]
fn test_cc_lost_packet() {
    let mut checker = CcChecker::new();

    // Первый пакет с payload и CC = 2
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 2);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::First);

    // Пропущен пакет с CC = 3, следующий пакет с CC = 4
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 4);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::Error { expected: 3, got: 4 });

    // Следующий пакет с CC = 5
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 5);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::Ok);
}

// Два PID вперемешку считаются независимо
#[test]
fn test_cc_two_pids() {
    let mut checker = CcChecker::new();

    // Первый пакет с PID = 256 и CC = 1
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 1);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::First);

    // Первый пакет с PID = 257 и CC = 5
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(257, 5);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::First);

    // Следующий пакет с PID = 256 и CC = 2
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 2);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::Ok);

    // Следующий пакет с PID = 257 и CC = 6
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(257, 6);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::Ok);

    // Следующий пакет с PID = 256 и CC = 4 вызывает ошибку, так как ожидался CC = 3
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 4);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::Error { expected: 3, got: 4 });

    // При этом пакет с PID = 257 и CC = 7 идёт нормально, так как ошибки были только для PID = 256
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(257, 7);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::Ok);

    // Следующий пакет с PID = 256 и CC = 5 идёт нормально, так как счетчик подстроился
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 5);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::Ok);
}

// После reset() следующий пакет снова First
#[test]
fn test_cc_reset() {
    let mut checker = CcChecker::new();

    // Первый пакет с PID = 256 и CC = 1
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 1);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::First);

    // Сброс состояния
    checker.reset();

    // Следующий пакет с PID = 256 и CC = 2 снова считается первым после сброса
    let mut data = [0u8; PACKET_SIZE];
    let mut packet = TsPacketMut::from(&mut data);
    packet.init(256, 2);
    packet.set_payload();

    assert_eq!(checker.check(&data), CcStatus::First);
}
