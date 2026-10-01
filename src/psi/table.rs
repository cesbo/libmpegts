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
        if section.len() <= 3 || section[1] & 0x80 == 0 {
            return Change::Ignored;
        }

        let table_id = section[0];
        let table_id_extension = u16::from_be_bytes([section[3], section[4]]);

        if self.is_empty {
            self.table_id = table_id;
            self.table_id_extension = table_id_extension;
            self.is_empty = false;

            return Change::Updated;
        }
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