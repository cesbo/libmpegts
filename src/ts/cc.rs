pub struct CcChecker {
    last_cc:[Option<u8>; 8192],
}

pub enum CcStatus {
    First,
    Ok,
    Discontinuity,
    Error { expected: u8, got: u8 }
}

impl CcChecker {
    pub fn check(&mut self, packet: &[u8]) -> CcStatus {
        let pid = ((packet[1] as u16 & 0x1F) << 8) | packet[2] as u16;

        if pid == 0x1FFF {
            return CcStatus::Ok;
        }

        let afc = (packet[3] >> 4) & 0x03;
        let cc  = packet[3] & 0x0F;

        let must_grow_cc: bool = afc & 0b01 != 0; // afc 01 или 11

        let has_af = afc & 0b10 != 0; // afc 10 или 11
        let di = has_af && packet[4] > 0 && packet[5] & 0x80 != 0;

        if di {
            self.last_cc[pid as usize] = Some(cc);
            return CcStatus::Discontinuity;
        }

        match self.last_cc[pid as usize] {
            None => {
                self.last_cc[pid as usize] = Some(cc);
                CcStatus::First
            }
            Some(last) if !must_grow_cc => {
                if cc == last {
                    CcStatus::Ok
                } else {
                    CcStatus::Error { expected: last, got: cc }
                }
            }
            Some(last) if must_grow_cc => { // TODO - посмотреть так ли оно по задаче
                let expected = (last + 1) % 16;
                if cc == expected {
                    self.last_cc[pid as usize] = Some(cc);
                    CcStatus::Ok
                } else {
                    CcStatus::Error { expected, got: cc }
                }
            }
            Some(last) => CcStatus::Error { expected: last, got: cc },
        }
    } 

    pub fn reset(&mut self) {
        self.last_cc = [None; 8192];
    }
}
