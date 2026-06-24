use crate::psi::{
    DescriptorRef,
    PsiSectionError,
};

/// short_event_descriptor (tag `0x4d`): the name of an event and a short
/// description, both as DVB-coded text, tagged with a 3-byte ISO 639 language
/// code.
#[derive(Debug, Clone, Copy)]
pub struct ShortEventDescriptorRef<'a>(&'a [u8]);

impl<'a> ShortEventDescriptorRef<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x4d;

    /// 3-byte ISO 639 language code.
    pub fn lang(&self) -> &'a [u8] {
        &self.0[0 .. 3]
    }

    /// Raw, DVB-coded event name bytes.
    pub fn event_name(&self) -> &'a [u8] {
        let len = self.0[3] as usize;
        let start = 4;
        let end = (start + len).min(self.0.len());
        &self.0[start .. end]
    }

    /// Raw, DVB-coded text bytes; empty when the optional text field is absent.
    pub fn text(&self) -> &'a [u8] {
        let name_len = self.0[3] as usize;
        let len_pos = 4 + name_len;
        if len_pos >= self.0.len() {
            return &[];
        }
        let len = self.0[len_pos] as usize;
        let start = len_pos + 1;
        let end = (start + len).min(self.0.len());
        &self.0[start .. end]
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for ShortEventDescriptorRef<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        // 3 language bytes + event_name_length.
        if data.len() < 4 {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        // The event name must fit. The trailing text field is treated as
        // optional so streams that omit it still yield a usable event name.
        let name_len = data[3] as usize;
        if 4 + name_len > data.len() {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(ShortEventDescriptorRef(data))
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
    fn parses_name_and_text() {
        let mut payload = b"eng".to_vec();
        payload.extend_from_slice(&field(b"Name"));
        payload.extend_from_slice(&field(b"Text"));
        let bytes = descriptor(0x4d, &payload);

        let se = ShortEventDescriptorRef::try_from(first(&bytes)).unwrap();
        assert_eq!(se.lang(), b"eng");
        assert_eq!(se.event_name(), b"Name");
        assert_eq!(se.text(), b"Text");
    }

    #[test]
    fn missing_text_field_is_lenient() {
        let mut payload = b"eng".to_vec();
        payload.extend_from_slice(&field(b"Name"));
        let bytes = descriptor(0x4d, &payload);

        let se = ShortEventDescriptorRef::try_from(first(&bytes)).unwrap();
        assert_eq!(se.event_name(), b"Name");
        assert_eq!(se.text(), b"");
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = descriptor(0x4e, b"eng\x00\x00");
        assert!(ShortEventDescriptorRef::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_overflowing_name() {
        // event_name_length = 10 but only 2 name bytes present
        let bytes = descriptor(0x4d, b"eng\x0aAB");
        assert!(ShortEventDescriptorRef::try_from(first(&bytes)).is_err());
    }
}
