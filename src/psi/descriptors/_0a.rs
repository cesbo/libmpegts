use textcode::StringDVB;


#[derive(Debug, Default)]
pub struct Language {
    /// Identifies the language or languages used by the associated program element.
    pub code: StringDVB,
    /// PID of the Transport Stream packets which shall contain
    /// either ECM or EMM information for the CA systems.
    pub audio_type: u8
}

impl Language {
    pub fn parse(slice: &[u8]) -> Self {
        Self {
            code: StringDVB::from(&slice[0 .. 3]),
            audio_type: slice[3]
        }
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        self.code.assemble(buffer);
        buffer.push(self.audio_type);
    }
}


/// The language descriptor is used to specify the language
/// of the associated program element
#[derive(Debug, Default)]
pub struct Desc0A {
    pub languages: Vec<Language>
}

impl Desc0A {
    #[inline]
    pub fn min_size() -> usize {
        2
    }

    pub fn check(slice: &[u8]) -> bool {
        slice.len() >= Self::min_size()
    }

    pub fn parse(slice: &[u8]) -> Self {
        let mut result = Self::default();
        
        let ptr = &slice[2 .. 2 + slice[1] as usize];
        let item_len = 4;

        let mut skip = 0;
        loop {
            let end = skip + item_len;
            if end > ptr.len() {
                break;
            }
            result.languages.push(Language::parse(&ptr[skip .. end]));
            skip = end;
        }

        result
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        buffer.push(0x0A);

        let skip = buffer.len();
        buffer.push(0x00);

        for lang in &self.languages {
            lang.assemble(buffer);
        }

        buffer[skip] = (buffer.len() - skip - 1) as u8;
    }
}
