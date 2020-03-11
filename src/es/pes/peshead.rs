#[derive(Default)]
pub struct PESHead {
    flag_error: bool,
    flag_start: bool,
    flag_priority: bool,
    pid: u16,
    scrembled: u8,
    adaptation_field_control: u8,
    cc: u8,
}
