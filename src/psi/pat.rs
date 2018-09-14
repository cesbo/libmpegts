use base::*;
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

        item.pnr = get_u16(&slice[0 ..]);
        item.pid = get_u16(&slice[2 ..]) & 0x1FFF;

        item
    }

    fn assmeble(&self, buffer: &mut Vec<u8>) {
        let skip = buffer.len();
        buffer.resize(skip + 4, 0x00);
        let ptr = buffer.as_mut_slice();
        set_u16(&mut ptr[skip + 0 ..], self.pnr);
        set_u16(&mut ptr[skip + 2 ..], 0xE000 + self.pid);
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

    /// Reads [`Psi`] and append data into the `Pat`
    ///
    /// # Examples
    ///
    /// ```
    /// use mpegts::psi::*;
    ///
    /// let mut packet: Vec<u8> = vec![
    ///     0x47, 0x40, 0x00, 0x10, 0x00, 0x00, 0xb0, 0x11,
    ///     0x00, 0x01, 0xc3, 0x00, 0x00, 0x00, 0x00, 0xe0,
    ///     0x10, 0x00, 0x01, 0xe4, 0x07, 0xe0, 0x44, 0xc6,
    ///     0x8d, /* ... */ ];
    /// packet.resize(188, 0xFF); // TS Packet should be 188 bytes
    ///
    /// let mut psi = Psi::default();
    /// psi.mux(&packet);
    /// assert!(psi.check());
    /// let mut pat = Pat::default();
    /// pat.parse(&psi);
    ///
    /// assert_eq!(pat.version, 1);
    /// assert_eq!(pat.tsid, 1);
    /// assert_eq!(pat.items.len(), 2);
    /// for item in pat.items.iter() {
    ///     match item.pnr {
    ///         0 => assert_eq!(item.pid, 16),
    ///         1 => assert_eq!(item.pid, 1031),
    ///         _ => unreachable!(),
    ///     };
    /// }
    /// ```
    pub fn parse(&mut self, psi: &Psi) {
        if ! self.check(&psi) {
            return;
        }

        self.version = psi.get_version();
        self.tsid = get_u16(&psi.buffer[3 ..]);

        let ptr = &psi.buffer[8 .. psi.size - 4];
        let mut skip = 0;
        while ptr.len() >= skip + 4 {
            self.items.push(PatItem::parse(&ptr[skip .. skip + 4]));
            skip += 4;
        }
    }

    /// Converts `Pat` into [`Psi`]
    ///
    /// # Examples
    ///
    /// ```
    /// use mpegts::psi::*;
    ///
    /// let mut pat = Pat::default();
    /// pat.version = 1;
    /// pat.tsid = 1;
    /// pat.items.push(PatItem { pnr: 0, pid: 16 });
    /// pat.items.push(PatItem { pnr: 1, pid: 1031 });
    /// let mut psi_custom = Psi::default();
    /// pat.assmeble(&mut psi_custom);
    ///
    /// let mut packet: Vec<u8> = vec![
    ///     0x47, 0x40, 0x00, 0x10, 0x00, 0x00, 0xb0, 0x11,
    ///     0x00, 0x01, 0xc3, 0x00, 0x00, 0x00, 0x00, 0xe0,
    ///     0x10, 0x00, 0x01, 0xe4, 0x07, 0xe0, 0x44, 0xc6,
    ///     0x8d, /* ... */ ];
    /// packet.resize(188, 0xFF); // TS Packet should be 188 bytes
    /// let mut psi_check = Psi::default();
    /// psi_check.mux(&packet);
    /// assert!(psi_check.check());
    /// assert_eq!(psi_custom, psi_check);
    /// assert_eq!(&psi_custom.buffer[.. psi_custom.size], &psi_check.buffer[.. psi_check.size]);
    /// ```
    pub fn assmeble(&self, psi: &mut Psi) {
        psi.init(0x00);
        psi.buffer.resize(8, 0x00);
        psi.set_version(self.version);
        set_u16(&mut psi.buffer[3 ..], self.tsid);

        for item in self.items.iter() {
            item.assmeble(&mut psi.buffer);
        }

        psi.finalize();
    }
}
