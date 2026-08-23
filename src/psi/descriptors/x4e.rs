use crate::psi::{
    DescriptorRef,
    PsiSectionError,
};

/// One item of an [`Desc4ERef`]: a description / value pair,
/// each as raw DVB-coded text.
#[derive(Debug, Clone, Copy)]
pub struct Desc4EItemRef<'a> {
    description: &'a [u8],
    item: &'a [u8],
}

impl<'a> Desc4EItemRef<'a> {
    /// Raw, DVB-coded item description bytes.
    pub fn item_description(&self) -> &'a [u8] {
        self.description
    }

    /// Raw, DVB-coded item bytes.
    pub fn item(&self) -> &'a [u8] {
        self.item
    }
}

/// Iterator over the items of an extended_event_descriptor.
pub struct Desc4EItemIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Iterator for Desc4EItemIter<'a> {
    type Item = Result<Desc4EItemRef<'a>, PsiSectionError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        let desc_len = self.data[self.offset] as usize;
        let desc_start = self.offset + 1;
        let desc_end = desc_start + desc_len;
        // The item_length byte must follow the description.
        if desc_end >= self.data.len() {
            self.offset = self.data.len();
            return Some(Err(PsiSectionError::InvalidDescriptorLength));
        }

        let item_len = self.data[desc_end] as usize;
        let item_start = desc_end + 1;
        let item_end = item_start + item_len;
        if item_end > self.data.len() {
            self.offset = self.data.len();
            return Some(Err(PsiSectionError::InvalidDescriptorLength));
        }

        let out = Desc4EItemRef {
            description: &self.data[desc_start .. desc_end],
            item: &self.data[item_start .. item_end],
        };
        self.offset = item_end;
        Some(Ok(out))
    }
}

/// extended_event_descriptor (tag `0x4e`): a detailed, possibly multi-part event
/// description, carrying an optional list of description/value item pairs and a
/// free text, tagged with a 3-byte ISO 639 language code.
#[derive(Debug, Clone, Copy)]
pub struct Desc4ERef<'a>(&'a [u8]);

impl<'a> Desc4ERef<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x4e;

    /// Number of this descriptor within the set (`0 ..= 15`).
    pub fn descriptor_number(&self) -> u8 {
        (self.0[0] & 0xf0) >> 4
    }

    /// Number of the last descriptor of the set (`0 ..= 15`).
    pub fn last_descriptor_number(&self) -> u8 {
        self.0[0] & 0x0f
    }

    /// 3-byte ISO 639 language code.
    pub fn lang(&self) -> &'a [u8] {
        &self.0[1 .. 4]
    }

    fn items_len(&self) -> usize {
        (self.0[4] as usize).min(self.0.len() - 5)
    }

    /// Iterator over the description/value item pairs.
    pub fn items(&self) -> Desc4EItemIter<'a> {
        let start = 5;
        let end = start + self.items_len();
        Desc4EItemIter {
            data: &self.0[start .. end],
            offset: 0,
        }
    }

    /// Raw, DVB-coded text bytes; empty when the optional text field is absent.
    pub fn text(&self) -> &'a [u8] {
        let len_pos = 5 + self.items_len();
        if len_pos >= self.0.len() {
            return &[];
        }
        let len = self.0[len_pos] as usize;
        let start = len_pos + 1;
        let end = (start + len).min(self.0.len());
        &self.0[start .. end]
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for Desc4ERef<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        // descriptor_number/last (1) + lang (3) + length_of_items (1).
        if data.len() < 5 {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        // The items block must fit; the trailing text field is optional.
        let items_len = data[4] as usize;
        if 5 + items_len > data.len() {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(Desc4ERef(data))
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

    fn field(s: &[u8]) -> Vec<u8> {
        let mut v = vec![s.len() as u8];
        v.extend_from_slice(s);
        v
    }

    #[test]
    fn parses_items_and_text() {
        let mut payload = vec![0x00]; // descriptor_number / last
        payload.extend_from_slice(b"eng");
        let mut items = field(b"Cast");
        items.extend_from_slice(&field(b"Bob"));
        payload.push(items.len() as u8);
        payload.extend_from_slice(&items);
        payload.extend_from_slice(&field(b"Body"));
        let bytes = descriptor(0x4e, &payload);

        let ee = Desc4ERef::try_from(first(&bytes)).unwrap();
        assert_eq!(ee.lang(), b"eng");
        assert_eq!(ee.text(), b"Body");
        let collected: Vec<_> = ee.items().map(Result::unwrap).collect();
        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].item_description(), b"Cast");
        assert_eq!(collected[0].item(), b"Bob");
    }

    #[test]
    fn empty_items_and_text() {
        let mut payload = vec![0x00];
        payload.extend_from_slice(b"eng");
        payload.push(0); // length_of_items
        let bytes = descriptor(0x4e, &payload);

        let ee = Desc4ERef::try_from(first(&bytes)).unwrap();
        assert_eq!(ee.items().count(), 0);
        assert_eq!(ee.text(), b"");
    }

    #[test]
    fn malformed_item_yields_error() {
        // length_of_items = 3, but the single item claims a 9-byte description.
        let mut payload = vec![0x00];
        payload.extend_from_slice(b"eng");
        payload.push(3);
        payload.extend_from_slice(&[0x09, 0x41, 0x42]);
        let bytes = descriptor(0x4e, &payload);

        let ee = Desc4ERef::try_from(first(&bytes)).unwrap();
        assert!(ee.items().next().unwrap().is_err());
    }
}
