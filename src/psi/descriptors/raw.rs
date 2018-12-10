/// Raw descriptor to store data if descriptor not supported in core
#[derive(Default, Debug)]
pub struct DescRaw {
    /// Descriptor tag
    pub tag: u8,
    /// Descriptor data
    pub data: Vec<u8>,
}

impl DescRaw {
    pub fn parse(slice: &[u8]) -> Self {
        DescRaw {
            tag: slice[0],
            data: {
                let mut data: Vec<u8> = Vec::new();
                let len = 2 + slice[1] as usize;
                data.extend_from_slice(&slice[2 .. len]);
                data
            },
        }
    }

    #[inline]
    pub fn size(&self) -> usize {
        2 + self.data.len()
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {
        if self.data.len() > 0xFF {
            return;
        }

        buffer.push(self.tag);
        buffer.push(self.data.len() as u8);
        buffer.extend_from_slice(&self.data);
    }
}
