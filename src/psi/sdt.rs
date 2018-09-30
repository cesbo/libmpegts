use psi;


#[derive(Debug, Default)]
pub struct SdtItem {
    // Program number.
    pub pnr: u16,
    // Indicates that EIT schedule information for the service is present in the current TS.
    pub eit_schedule_flag: u8,
    // Indicates that EIT_present_following information for the service is present in the current TS.
    pub eit_present_following_flag: u8,
    // Indicating the status of the service.
    pub running_status: u8,
    // Indicates that all the component streams of the service are not scrambled.
    pub free_ca_mode: u8,
    // List of descriptors.
    pub descriptors: psi::descriptors::Descriptor
}

impl SdtItem {
    fn parse(slice: &[8]) -> Self {}

    fn assemble(&self, buffer: &mut Vec<u8>) {}
}

#[derive(Debug, Default)]
pub struct Sdt {
    // Identifies to which table the section belongs:
    // * `0x42` - actual TS
    // * `0x46` - other TS
    pub table_id: u8,
    // Transport stream identifier.
    pub tsid: u16,
    // SDT version.
    pub version: u8,
    // Identifying the network of the originating delivery system.
    pub onid: u16,
    // List of SDT items.
    pub items: Vec<SdtItem>
}

#[derive(Debug, Default)]
impl Sdt {
    #[inline]
    fn check(&self, psi: &psi::Psi) -> bool {
        // TODO: check for psi.size
        match psi.buffer[0] {
            0x42 => true,  /* actual TS */
            0x46 => true,  /* other TS */
            _ => false
        } &&
        psi.check()
    }

    pub fn parse(&mut self, psi: &psi::Psi) {}

    pub fn assemble(&self, psiL &mut psi::Psi) {}
}
