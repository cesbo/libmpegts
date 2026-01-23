/// Reference to a single MPEG-TS descriptor (tag + length + data).
#[derive(Debug, Clone, Copy)]
pub struct DescriptorRef<'a>(&'a [u8]);

impl<'a> DescriptorRef<'a> {
    pub fn tag(&self) -> u8 {
        self.0[0]
    }
    pub fn data(&self) -> &'a [u8] {
        &self.0[2 ..]
    }
}

/// Reference to a list of MPEG-TS descriptors.
pub struct DescriptorsRef<'a>(&'a [u8]);

impl<'a> From<&'a [u8]> for DescriptorsRef<'a> {
    fn from(value: &'a [u8]) -> Self {
        DescriptorsRef(value)
    }
}

impl<'a> IntoIterator for DescriptorsRef<'a> {
    type Item = DescriptorRef<'a>;
    type IntoIter = DescriptorIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        DescriptorIter {
            data: self.0,
            offset: 0,
        }
    }
}

/// Iterator over MPEG-TS descriptors.
pub struct DescriptorIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for DescriptorIter<'a> {
    type Item = DescriptorRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 2 > self.data.len() {
            return None;
        }
        let end = self.offset + 2 + self.data[self.offset + 1] as usize;
        if end > self.data.len() {
            return None;
        }
        let desc = DescriptorRef(&self.data[self.offset .. end]);
        self.offset = end;
        Some(desc)
    }
}
