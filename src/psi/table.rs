pub struct SectionTable {
    table_id: u8,
    table_id_extension: u16,
    version: u8,
    last_section_number: u8,
    crc: [Option<u32>; 256],
    is_empty: bool,
}

pub enum Change { 
    Ignored, 
    Unchanged, 
    Updated, 
    Reset
}