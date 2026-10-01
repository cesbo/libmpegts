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

impl SectionTable {
    pub fn push(&mut self, section: &[u8]) -> Change {
        // Implementation goes here
        Change::Ignored
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

impl Default for SectionTable {
    fn default() -> Self {
        Self {
            table_id: 0,
            table_id_extension: 0,
            version: 0,
            last_section_number: 0,
            crc: [None; 256],
            is_empty: true,
        }
    }
}