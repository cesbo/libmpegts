use super::{PACKET_SIZE, PID_NONE, PID_NULL, TsPacketRef};

const PID_COUNT: usize = PID_NONE as usize;

pub struct CcChecker {
    last_cc: [Option<u8>; PID_COUNT],
}

#[derive(Debug, PartialEq)]
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

        let afc = (packet[3] >> 4) & 0x03;
        let cc = ts.cc();

        let must_grow_cc: bool = afc & 0b01 != 0; // afc 01 или 11

        // 2.4.3.4, Table 2-6 «Transport stream adaptation field» H.222.0 (10/14)
        let di = ts
            .adaptation_field()
            .is_some_and(|af| af.discontinuity_indicator());

        if di {
            self.last_cc[idx] = Some(cc);
            return CcStatus::Discontinuity;
        }

        match self.last_cc[idx] {
            None => {
                self.last_cc[idx] = Some(cc);
                CcStatus::First
            }
            Some(last) => {
                let expected = if must_grow_cc {
                    (last + 1) & 0x0F
                } else {
                    last
                };
                self.last_cc[idx] = Some(cc);

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