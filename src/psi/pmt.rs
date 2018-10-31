use psi::{Psi, Descriptors};


pub const PMT_PID: u16 = 0x02;


#[derive(Debug, Default)]
pub struct PmtItem {
    pub stream_type: u8,
    pub pid: u16,
    pub descriptors: Descriptors
}

impl PmtItem {
    pub fn parse(slice: &[u8]) -> Self {
        Self::default()
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {

    }
}


#[derive(Debug, Default)]
pub struct Pmt {
    pub version: u8,
    pub pnr: u16,
    pub pcr: u16,
    pub descriptors: Descriptors,
    pub items: Vec<PmtItem>
}

impl Pmt {
    pub fn check(&self, psi: &Psi) -> bool {
        true
    }

    pub fn parse(&mut self, psi: &Psi) {

    }

    pub fn assemble(&self, psi: &mut Psi) {

    }
}
