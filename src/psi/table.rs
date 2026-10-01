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

        let section_number = section[6];
        let last_section_number = section[7];

        // iso13818-1 2.4.4.11: last_section_number is the highest section_number
        // of the table, so a greater section_number means a broken section
        // (corrupted data or a faulty multiplexer). Ignore it so that it does not
        // leave a stale crc slot beyond last_section_number.
        if section_number > last_section_number {
            return Change::Ignored;
        }

        let table_id = section[0];
        let table_id_extension = u16::from_be_bytes([section[3], section[4]]);
        let version = (section[5] >> 1) & 0x1F;
        let section_length = (usize::from(section[1] & 0x0F) << 8) | usize::from(section[2]); 
        let end = 3 + section_length;
        let crc = u32::from_be_bytes(section[end - 4 .. end].try_into().unwrap());

        if self.is_empty {
            self.table_id = table_id;
            self.table_id_extension = table_id_extension;
            self.version = version;
            self.last_section_number = last_section_number;
            self.crc[usize::from(section_number)] = Some(crc);
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