use crate::psi::{
    DescriptorRef,
    PsiSectionError,
};

/// One entry of an [`Desc0ARef`]: a 3-byte ISO 639 language
/// code plus audio type.
#[derive(Debug, Clone, Copy)]
pub struct Desc0AItemRef<'a> {
    lang: &'a [u8],
    audio_type: u8,
}

impl<'a> Desc0AItemRef<'a> {
    /// 3-byte ISO 639 language code.
    pub fn lang(&self) -> &'a [u8] {
        self.lang
    }

    /// Audio type byte.
    pub fn audio_type(&self) -> u8 {
        self.audio_type
    }
}

/// Iterator over the entries of an ISO_639_language_descriptor.
pub struct Desc0AItemIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for Desc0AItemIter<'a> {
    type Item = Desc0AItemRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset + 4 > self.data.len() {
            return None;
        }
        let out = Desc0AItemRef {
            lang: &self.data[self.offset .. self.offset + 3],
            audio_type: self.data[self.offset + 3],
        };
        self.offset += 4;
        Some(out)
    }
}

/// ISO_639_language_descriptor (tag `0x0a`): one or more ISO 639 language
/// records, each carrying a 3-byte language code and an audio type byte.
#[derive(Debug, Clone, Copy)]
pub struct Desc0ARef<'a>(&'a [u8]);

impl<'a> Desc0ARef<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x0a;

    /// Iterator over language/audio-type records.
    pub fn items(&self) -> Desc0AItemIter<'a> {
        Desc0AItemIter {
            data: self.0,
            offset: 0,
        }
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for Desc0ARef<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        if !data.len().is_multiple_of(4) {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(Desc0ARef(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::psi::DescriptorsRef;

    fn descriptor(tag: u8, payload: &[u8]) -> Vec<u8> {
        let mut v = vec![tag, payload.len() as u8];
        v.extend_from_slice(payload);
        v
    }

    fn first(bytes: &[u8]) -> DescriptorRef<'_> {
        DescriptorsRef::from(bytes)
            .into_iter()
            .next()
            .unwrap()
            .unwrap()
    }

    #[test]
    fn parses_language_items() {
        let bytes = descriptor(0x0a, b"eng\x01deu\x02");

        let iso = Desc0ARef::try_from(first(&bytes)).unwrap();
        let items: Vec<_> = iso.items().collect();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].lang(), b"eng");
        assert_eq!(items[0].audio_type(), 1);
        assert_eq!(items[1].lang(), b"deu");
        assert_eq!(items[1].audio_type(), 2);
    }

    #[test]
    fn accepts_empty_payload() {
        let bytes = descriptor(0x0a, b"");

        let iso = Desc0ARef::try_from(first(&bytes)).unwrap();
        assert_eq!(iso.items().count(), 0);
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = descriptor(0x09, b"eng\x01");
        assert!(Desc0ARef::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_partial_item() {
        let bytes = descriptor(0x0a, b"eng\x01x");
        assert!(Desc0ARef::try_from(first(&bytes)).is_err());
    }
}
