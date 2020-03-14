use bitwrap::BitWrap;


#[derive(BitWrap, Default, Debug)]
pub struct TSHead {
    #[bits(8)]
    sync: u8,
    #[bits(1)]
    pub flag_error: bool,
    #[bits(1)]
    pub flag_payload_start: bool,
    #[bits(1)]
    flag_priority: bool,
    #[bits(13)]
    pub pid: u16,
    #[bits(2)]
    pub scrambled: u8,
    #[bits(2)]
    pub adaptation_field_control: u8,
    #[bits(4)]
    pub cc: u8,
}


impl TSHead {
    pub fn is_sync(&self) -> bool {
        self.sync == 0x47
    }

    pub fn is_payload(&self) -> bool {
        (self.adaptation_field_control & 0x01) != 0x00
    }

    pub fn is_adaptation(&self) -> bool {
        (self.adaptation_field_control & 0x02) != 0x00
    }
}
