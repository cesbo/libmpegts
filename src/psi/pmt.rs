use base;
use psi::{Psi, Descriptors};

/// PMT Item.
#[derive(Debug, Default)]
pub struct PmtItem {
    /// This field specifying the type of program element
    /// carried within the packets with the PID.
    pub stream_type: u8,
    /// This field specifying the PID of the Transport Stream packets
    /// which carry the associated program element.
    pub pid: u16,
    /// List of descriptors.
    pub descriptors: Descriptors
}

impl PmtItem {
    pub fn parse(slice: &[u8]) -> Self {
        let mut item = Self::default();

        item.stream_type = slice[0];
        item.pid = base::get_pid(&slice[1 ..]);

        item.descriptors.parse(&slice[5 ..]);

        item
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();
        buffer.resize(skip + 5, 0x00);

        buffer[skip] = self.stream_type;
        base::set_pid(&mut buffer[skip + 1 ..], self.pid);

        self.descriptors.assemble(buffer);

        let descs_len = buffer.len() - skip - 5;
        if descs_len > 0 {
            base::set_u12(&mut buffer[skip + 3 ..], descs_len as u16);
        }
    }

    #[inline]
    fn size(&self) -> usize {
        5 + self.descriptors.size()
    }
}

/// Program Map Table - provides the mappings between program numbers
/// and the program elements that comprise them.
#[derive(Debug, Default)]
pub struct Pmt {
    /// PMT version.
    pub version: u8,
    /// Program number.
    pub pnr: u16,
    /// PCR (Program Clock Reference) pid.
    pub pcr: u16,
    /// List of descriptors.
    pub descriptors: Descriptors,
    /// List of PMT items.
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
        self.pcr = base::get_pid(&psi.buffer[8 ..]);

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

    fn psi_init(&self, first: bool) -> Psi {
        let mut psi = Psi::default();
        psi.init(0x02);
        psi.buffer.resize(12, 0x00);
        psi.set_version(self.version);
        base::set_u16(&mut psi.buffer[3 ..], self.pnr);
        base::set_pid(&mut psi.buffer[8 ..], self.pcr);
        psi.buffer[10] = 0xF0;  //reserved
        if first {
            self.descriptors.assemble(&mut psi.buffer);
            let len = (psi.buffer.len() - 12) as u16;
            base::set_u12(&mut psi.buffer[10 ..], len);
        }
        psi
    }

    #[inline]
    fn psi_max_size(&self) -> usize {
        1024
    }

    /// Converts `Pmt` into TS packets
    pub fn demux(&self, pid: u16, cc: &mut u8, dst: &mut Vec<u8>) {
        let mut psi_list = Vec::<Psi>::new();
        let psi = self.psi_init(true);
        let mut psi_size = psi.buffer.len();
        psi_list.push(psi);

        let mut psi_size = 0;
        for item in &self.items {
            if self.psi_max_size() >= psi_size + item.size() {
                let mut psi = psi_list.last_mut().unwrap();
                item.assemble(&mut psi.buffer);
                psi_size = psi.buffer.len();
            } else {
                let mut psi = self.psi_init(false);
                item.assemble(&mut psi.buffer);
                psi_size = psi.buffer.len();
                psi_list.push(psi);
            }
        }

        let mut section_number: u8 = 0;
        let last_section_number = (psi_list.len() - 1) as u8;
        for psi in &mut psi_list {
            psi.buffer[6] = section_number;
            psi.buffer[7] = last_section_number;
            psi.finalize();

            section_number += 1;

            psi.pid = pid;
            psi.cc = *cc;
            psi.demux(dst);
            *cc = psi.cc;
        }
    }
}
