use super::{PACKET_SIZE, PID_NONE, PID_NULL, TsPacketRef};

const PID_COUNT: usize = PID_NONE as usize;

pub struct CcChecker {
    last_cc: [Option<u8>; PID_COUNT],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CcStatus {
    First,
    Ok,
    Discontinuity,
    Error { expected: u8, got: u8 },
}

impl CcChecker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn check(&mut self, packet: &[u8]) -> CcStatus {
        let packet: &[u8; PACKET_SIZE] = packet.try_into().unwrap();
        let ts = TsPacketRef::from(packet);

        let pid = ts.pid();

        if pid == PID_NULL {
            return CcStatus::Ok;
        }

        let idx = usize::from(pid);

        let has_payload = packet[3] & 0x10 != 0; // afc x1: есть payload
        let has_af = packet[3] & 0x20 != 0; // afc 1x: есть adaptation field

        // afc 00 зарезервирован, такой пакет отбрасывается
        if !has_payload && !has_af {
            return CcStatus::Ok;
        }

        let cc = ts.cc();

        // 2.4.3.4, Table 2-6 «Transport stream adaptation field» H.222.0 (10/14)
        let di = ts
            .adaptation_field()
            .is_some_and(|af| af.discontinuity_indicator());

        // Записывает новый CC и возвращает прежний
        let last = self.last_cc[idx].replace(cc);

        if di {
            return CcStatus::Discontinuity;
        }

        match last {
            None => CcStatus::First,
            Some(last) => {
                let expected = if has_payload {
                    (last + 1) & 0x0F
                } else {
                    last
                };

                if cc == expected {
                    CcStatus::Ok
                } else {
                    CcStatus::Error { expected, got: cc }
                }
            }
        }
    }

    pub fn reset(&mut self) {
        self.last_cc = [None; PID_COUNT];
    }
}

impl Default for CcChecker {
    fn default() -> Self {
        Self { last_cc: [None; PID_COUNT] }
    }
}