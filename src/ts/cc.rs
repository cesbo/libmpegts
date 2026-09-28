pub struct CcChecker {}

pub enum CcStatus {
    First,
    Ok,
    Discontinuity,
    Error { expected: u8, got: u8 }
}

impl CcChecker {
    pub fn check(&self, packet: &[u8]) -> CcStatus {} 

    pub fn reset(&mut self) {}
}