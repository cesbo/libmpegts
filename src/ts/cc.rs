use super::{PID_NONE, PID_NULL};

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
        let pid = ((packet[1] as u16 & 0x1F) << 8) | packet[2] as u16;

        if pid == PID_NULL {
            return CcStatus::Ok;
        }

        let idx = usize::from(pid);

        let afc = (packet[3] >> 4) & 0x03;
        let cc = packet[3] & 0x0F;

        let must_grow_cc: bool = afc & 0b01 != 0; // afc 01 или 11

        let has_af = afc & 0b10 != 0; // afc 10 или 11
        let di = has_af && packet[4] > 0 && packet[5] & 0x80 != 0; // 2.4.3.4, Table 2-6 «Transport stream adaptation field» H.222.0 (10/14)

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