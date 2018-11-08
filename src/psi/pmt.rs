use base;
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
        let mut item = Self::default();

        item.stream_type = slice[0];
        item.pid = base::get_u13(&slice[1 ..]);

        item.descriptors.parse(&slice[5 ..]);

        item
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
    #[inline]
    pub fn check(&self, psi: &Psi) -> bool {
        psi.size >= 12 + 4 &&
        match psi.buffer[0] {
            0x02 => true,
            _ => false
        } &&
        psi.check()
    }

    pub fn parse(&mut self, psi: &Psi) {
        if ! self.check(psi) {
            return;
        }

        self.version = psi.get_version();
        self.pnr = base::get_u16(&psi.buffer[3 ..]);
        self.pcr = base::get_u13(&psi.buffer[8 ..]);

        let program_length = base::get_u12(&psi.buffer[10 ..]) as usize;
        self.descriptors.parse(&psi.buffer[11 .. 11 + program_length]);

        let ptr = &psi.buffer[12 + program_length .. psi.size - 4];
        let mut skip = 0;
        while ptr.len() >= skip + 5 {
            let item_len = 5 + base::get_u12(&ptr[skip + 3 ..]) as usize;
            if skip + item_len > ptr.len() {
                break;
            }
            self.items.push(PmtItem::parse(&ptr[skip .. skip + item_len]));
            skip += item_len;
        }
    }

    pub fn assemble(&self, psi: &mut Psi) {

    }
}
