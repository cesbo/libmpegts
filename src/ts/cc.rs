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
    pub fn check(&self, packet: &[u8]) -> CcStatus {
        let pid = ((packet[1] as u16 & 0x1F) << 8) | packet[2] as u16;

        if pid == 0x1FFF {
            return CcStatus::Ok;
        }

        let cc  = packet[3] & 0x0F;
    } 

    pub fn reset(&mut self) {}
}