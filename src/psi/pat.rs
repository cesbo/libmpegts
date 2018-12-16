use base;
use psi::Psi;

/// TS Packet Identifier for PAT
pub const PAT_PID: u16 = 0x00;

/// PAT Item
#[derive(Debug, Default, PartialEq)]
pub struct PatItem {
    /// Program Number
    pub pnr: u16,
    /// TS Packet Idetifier
    pub pid: u16,
}

impl PatItem {
    fn parse(slice: &[u8]) -> Self {
        let mut item = PatItem::default();

        item.pnr = base::get_u16(&slice[0 ..]);
        item.pid = base::get_u16(&slice[2 ..]) & 0x1FFF;

        item
    }

    fn assemble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();
        buffer.resize(skip + 4, 0x00);
        base::set_u16(&mut buffer[skip ..], self.pnr);
        base::set_pid(&mut buffer[skip + 2 ..], self.pid);
    }

    #[inline]
    fn size(&self) -> usize {
        4
    }
}

/// Program Association Table provides the correspondence between a `pnr` (Program Number) and
/// the `pid` value of the TS packets which carry the program definition.
#[derive(Default, Debug, PartialEq)]
pub struct Pat {
    /// PAT version
    pub version: u8,
    /// Transport Stream ID to identify actual stream from any other multiplex within a network
    pub tsid: u16,
    /// List of the PAT Items
    pub items: Vec<PatItem>,
}

impl Pat {
    #[inline]
    fn check(&self, psi: &Psi) -> bool {
        psi.size >= 8 + 4 &&
        match psi.buffer[0] {
            0x00 => true,
            _ => false,
        } &&
        psi.check()

        // TODO: check if PSI already parsed
    }

    /// Reads PSI packet and append data into the `Pat`
    pub fn parse(&mut self, psi: &Psi) {
        if ! self.check(&psi) {
            return;
        }

        self.version = psi.get_version();
        self.tsid = base::get_u16(&psi.buffer[3 ..]);

        let ptr = &psi.buffer[8 .. psi.size - 4];
        let mut skip = 0;
        while ptr.len() >= skip + 4 {
            self.items.push(PatItem::parse(&ptr[skip .. skip + 4]));
            skip += 4;
        }
    }

    fn psi_init(&self, _first: bool) -> Psi {
        let mut psi = Psi::default();
        psi.init(0x00);
        psi.buffer.resize(8, 0x00);
        psi.set_version(self.version);
        base::set_u16(&mut psi.buffer[3 ..], self.tsid);
        psi
    }

    #[inline]
    fn psi_max_size(&self) -> usize {
        1024
    }

    /// Converts `Pat` into TS packets
    pub fn demux(&self, cc: &mut u8, dst: &mut Vec<u8>) {
        let mut psi_list = Vec::<Psi>::new();
        let psi = self.psi_init(true);
        let mut psi_size = psi.buffer.len();
        psi_list.push(psi);

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

            psi.pid = PAT_PID;
            psi.cc = *cc;
            psi.demux(dst);
            *cc = psi.cc;
        }
    }
}
