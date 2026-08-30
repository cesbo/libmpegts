use crate::{
    psi::{
        DescriptorRef,
        PsiSectionError,
    },
    utils::textcode::{
        self,
        DvbTextRef,
    },
};

/// One entry of a [`Desc5BRef`]: a network name in one language.
#[derive(Debug, Clone, Copy)]
pub struct Desc5BItemRef<'a> {
    lang: &'a [u8],
    name: &'a [u8],
}

impl<'a> Desc5BItemRef<'a> {
    /// 3-byte ISO 639 language code.
    pub fn lang(&self) -> &'a [u8] {
        self.lang
    }

    /// Raw, DVB-coded network name bytes.
    pub fn name(&self) -> &'a [u8] {
        self.name
    }

    /// Network name decoded according to DVB text coding.
    pub fn name_text(&self) -> Result<DvbTextRef<'a>, textcode::Error> {
        DvbTextRef::try_from(self.name)
    }
}

/// Iterator over the entries of a multilingual_network_name_descriptor.
pub struct Desc5BItemIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for Desc5BItemIter<'a> {
    type Item = Result<Desc5BItemRef<'a>, PsiSectionError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        if self.offset + 4 > self.data.len() {
            self.offset = self.data.len();
            return Some(Err(PsiSectionError::InvalidDescriptorLength));
        }

        let name_len = self.data[self.offset + 3] as usize;
        let name_start = self.offset + 4;
        let name_end = name_start + name_len;
        if name_end > self.data.len() {
            self.offset = self.data.len();
            return Some(Err(PsiSectionError::InvalidDescriptorLength));
        }

        let out = Desc5BItemRef {
            lang: &self.data[self.offset .. self.offset + 3],
            name: &self.data[name_start .. name_end],
        };
        self.offset = name_end;
        Some(Ok(out))
    }
}

/// multilingual_network_name_descriptor (tag `0x5b`): the network name in one
/// or more languages.
#[derive(Debug, Clone, Copy)]
pub struct Desc5BRef<'a>(&'a [u8]);

impl<'a> Desc5BRef<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x5b;

    /// Iterator over per-language network names.
    pub fn items(&self) -> Desc5BItemIter<'a> {
        Desc5BItemIter {
            data: self.0,
            offset: 0,
        }
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for Desc5BRef<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        Ok(Desc5BRef(descriptor.data()))
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
    fn parses_multilingual_names() {
        let mut bytes = vec![0x5b, 0x13];
        bytes.extend_from_slice(b"eng\x05Astra");
        bytes.extend_from_slice(b"deu\x06Astra2");

        let network = Desc5BRef::try_from(first(&bytes)).unwrap();
        let items: Vec<_> = network.items().collect::<Result<_, _>>().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].lang(), b"eng");
        assert_eq!(items[0].name_text().unwrap().to_string(), "Astra");
        assert_eq!(items[1].lang(), b"deu");
        assert_eq!(items[1].name_text().unwrap().to_string(), "Astra2");
    }

    #[test]
    fn accepts_empty_payload() {
        let bytes = [0x5b, 0x00];

        let network = Desc5BRef::try_from(first(&bytes)).unwrap();
        assert_eq!(network.items().count(), 0);
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = [0x40, 0x04, 0x65, 0x6e, 0x67, 0x00];
        assert!(Desc5BRef::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn truncated_name_yields_error_item() {
        // Declared name length 5, only 2 bytes present
        let bytes = [0x5b, 0x06, 0x65, 0x6e, 0x67, 0x05, 0x41, 0x73];

        let network = Desc5BRef::try_from(first(&bytes)).unwrap();
        let mut iter = network.items();
        assert!(iter.next().unwrap().is_err());
        assert!(iter.next().is_none());
    }
}
