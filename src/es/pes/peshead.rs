use bitwrap::BitWrap;


#[derive(BitWrap, Default, Debug)]
pub struct PESHead {
    #[bits(1)]
    flag_error: bool,
    #[bits(1)]
    flag_start: bool,
    #[bits(1)]
    flag_priority: bool,
    #[bits(13)]
    pid: u16,
    #[bits(2)]
    scrembled: u8,
    #[bits(2)]
    adaptation_field_control: u8,
    #[bits(4)]
    cc: u8,
}
