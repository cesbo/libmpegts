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

    fn assmeble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();
        buffer.resize(skip + 4, 0x00);
        base::set_u16(&mut buffer[skip ..], self.pnr);
        base::set_u16(&mut buffer[skip + 2 ..], 0xE000 + self.pid);
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

    /// Converts `Pat` into PSI
    pub fn assmeble(&self, psi: &mut Psi) {
        psi.init(0x00);
        psi.buffer.resize(8, 0x00);
        psi.set_version(self.version);
        base::set_u16(&mut psi.buffer[3 ..], self.tsid);

        for item in &self.items {
            item.assmeble(&mut psi.buffer);
        }

        psi.finalize();
    }
}
