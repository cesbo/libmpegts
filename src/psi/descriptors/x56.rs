use crate::psi::{
    DescriptorRef,
    PsiSectionError,
};

/// One entry of a [`Desc56Ref`]: a language code plus teletext page
/// reference.
#[derive(Debug, Clone, Copy)]
pub struct Desc56ItemRef<'a> {
    lang: &'a [u8],
    type_magazine: u8,
    page_number: u8,
}

impl<'a> Desc56ItemRef<'a> {
    /// 3-byte ISO 639 language code.
    pub fn lang(&self) -> &'a [u8] {
        self.lang
    }

    /// Teletext type:
    /// * `0x01` - initial page
    /// * `0x02` - subtitles
    /// * `0x03` - additional information
    /// * `0x04` - programme schedule
    /// * `0x05` - hearing-impaired subtitles
    pub fn teletext_type(&self) -> u8 {
        (self.type_magazine & 0xf8) >> 3
    }

    /// Teletext magazine number.
    pub fn magazine_number(&self) -> u8 {
        self.type_magazine & 0x07
    }

    /// Teletext page number as two 4-bit digits (tens, units).
    pub fn page_number(&self) -> u8 {
        self.page_number
    }
}

/// Iterator over the entries of a teletext_descriptor.
pub struct Desc56ItemIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for Desc56ItemIter<'a> {
    type Item = Desc56ItemRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 5 > self.data.len() {
            return None;
        }
        let out = Desc56ItemRef {
            lang: &self.data[self.offset .. self.offset + 3],
            type_magazine: self.data[self.offset + 3],
            page_number: self.data[self.offset + 4],
        };
        self.offset += 5;
        Some(out)
    }
}

/// teletext_descriptor (tag `0x56`): teletext pages carried in the EBU
/// teletext elementary stream.
#[derive(Debug, Clone, Copy)]
pub struct Desc56Ref<'a>(&'a [u8]);

impl<'a> Desc56Ref<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x56;

    /// Iterator over teletext page entries.
    pub fn items(&self) -> Desc56ItemIter<'a> {
        Desc56ItemIter {
            data: self.0,
            offset: 0,
        }
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for Desc56Ref<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        if !data.len().is_multiple_of(5) {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(Desc56Ref(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psi::DescriptorsRef;

    fn first(bytes: &[u8]) -> DescriptorRef<'_> {
        DescriptorsRef::from(bytes)
            .into_iter()
            .next()
            .unwrap()
            .unwrap()
    }

    #[test]
    fn parses_teletext_items() {
        // initial page 100 (magazine 1, page 0x00), subtitles 888 (magazine 0, page 0x88)
        let bytes = [
            0x56, 0x0a, 0x64, 0x65, 0x75, 0x09, 0x00, 0x64, 0x65, 0x75, 0x10, 0x88,
        ];

        let teletext = Desc56Ref::try_from(first(&bytes)).unwrap();
        let items: Vec<_> = teletext.items().collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].lang(), b"deu");
        assert_eq!(items[0].teletext_type(), 1);
        assert_eq!(items[0].magazine_number(), 1);
        assert_eq!(items[0].page_number(), 0x00);
        assert_eq!(items[1].teletext_type(), 2);
        assert_eq!(items[1].magazine_number(), 0);
        assert_eq!(items[1].page_number(), 0x88);
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = [0x59, 0x05, 0x64, 0x65, 0x75, 0x09, 0x00];
        assert!(Desc56Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_partial_item() {
        let bytes = [0x56, 0x04, 0x64, 0x65, 0x75, 0x09];
        assert!(Desc56Ref::try_from(first(&bytes)).is_err());
    }
}
