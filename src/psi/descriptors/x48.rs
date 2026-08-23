use crate::{
    psi::{
        Descriptor,
        DescriptorRef,
        PsiSectionError,
    },
    utils::textcode::{
        Charset,
        TextcodeError,
        TextcodeRef,
    },
};

/// service_descriptor (tag `0x48`): service type, provider name and service
/// name as defined by DVB SI.
#[derive(Debug, Clone, Copy)]
pub struct Desc48Ref<'a>(&'a [u8]);

impl<'a> Desc48Ref<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x48;

    /// Service type byte.
    pub fn service_type(&self) -> u8 {
        self.0[0]
    }

    fn provider_name_len(&self) -> usize {
        self.0[1] as usize
    }

    fn service_name_len_pos(&self) -> usize {
        2 + self.provider_name_len()
    }

    /// Raw, DVB-coded service provider name bytes.
    pub fn provider_name(&self) -> &'a [u8] {
        let start = 2;
        let end = start + self.provider_name_len();
        &self.0[start .. end]
    }

    /// Service provider name decoded according to DVB text coding.
    pub fn provider_name_text(&self) -> Result<TextcodeRef<'a>, TextcodeError> {
        TextcodeRef::try_from(self.provider_name())
    }

    /// Raw, DVB-coded service name bytes.
    pub fn service_name(&self) -> &'a [u8] {
        let len_pos = self.service_name_len_pos();
        let len = self.0[len_pos] as usize;
        let start = len_pos + 1;
        let end = start + len;
        &self.0[start .. end]
    }

    /// Service name decoded according to DVB text coding.
    pub fn service_name_text(&self) -> Result<TextcodeRef<'a>, TextcodeError> {
        TextcodeRef::try_from(self.service_name())
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for Desc48Ref<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        // service_type + service_provider_name_length + service_name_length.
        if data.len() < 3 {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }

        let provider_name_len = data[1] as usize;
        let service_name_len_pos = 2 + provider_name_len;
        if service_name_len_pos >= data.len() {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }

        let service_name_len = data[service_name_len_pos] as usize;
        let service_name_end = service_name_len_pos + 1 + service_name_len;
        if service_name_end != data.len() {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }

        Ok(Desc48Ref(data))
    }
}

/// service_descriptor (tag `0x48`) encoder. Provider and service names are
/// DVB-coded with `charset`.
#[derive(Debug, Clone, Copy)]
pub struct Desc48<'a> {
    pub service_type: u8,
    pub provider_name: &'a str,
    pub service_name: &'a str,
    pub charset: Charset,
}

impl Descriptor for Desc48<'_> {
    fn encode(&self, dst: &mut Vec<u8>) -> Result<(), PsiSectionError> {
        let provider_name = textcode::dvb::encode(self.provider_name, self.charset);
        let service_name = textcode::dvb::encode(self.service_name, self.charset);

        let data_len = 3 + provider_name.len() + service_name.len();
        if data_len > 0xff {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }

        dst.push(Desc48Ref::TAG);
        dst.push(data_len as u8);
        dst.push(self.service_type);
        dst.push(provider_name.len() as u8);
        dst.extend_from_slice(&provider_name);
        dst.push(service_name.len() as u8);
        dst.extend_from_slice(&service_name);
        Ok(())
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
    fn parses_service_fields() {
        let bytes = descriptor(0x48, b"\x01\x06Avalpa\x04Name");

        let service = Desc48Ref::try_from(first(&bytes)).unwrap();
        assert_eq!(service.service_type(), 1);
        assert_eq!(service.provider_name(), b"Avalpa");
        assert_eq!(service.service_name(), b"Name");
        assert_eq!(service.provider_name_text().unwrap().to_string(), "Avalpa");
        assert_eq!(service.service_name_text().unwrap().to_string(), "Name");
    }

    #[test]
    fn accepts_empty_names() {
        let bytes = descriptor(0x48, b"\x01\x00\x00");

        let service = Desc48Ref::try_from(first(&bytes)).unwrap();
        assert_eq!(service.provider_name(), b"");
        assert_eq!(service.service_name(), b"");
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = descriptor(0x49, b"\x01\x00\x00");
        assert!(Desc48Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_overflowing_provider_name() {
        let bytes = descriptor(0x48, b"\x01\x06Aval");
        assert!(Desc48Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_overflowing_service_name() {
        let bytes = descriptor(0x48, b"\x01\x00\x04Na");
        assert!(Desc48Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_trailing_bytes() {
        let bytes = descriptor(0x48, b"\x01\x00\x00x");
        assert!(Desc48Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn encodes_iso6937_names_without_header() {
        let mut dst = Vec::new();
        Desc48 {
            service_type: 1,
            provider_name: "Avalpa",
            service_name: "Name",
            charset: Charset::Iso6937,
        }
        .encode(&mut dst)
        .unwrap();

        assert_eq!(dst, b"\x48\x0d\x01\x06Avalpa\x04Name");
    }

    #[test]
    fn encodes_names_with_charset_header() {
        let mut dst = Vec::new();
        Desc48 {
            service_type: 1,
            provider_name: "Провайдер",
            service_name: "Канал",
            charset: Charset::Iso8859_5,
        }
        .encode(&mut dst)
        .unwrap();

        let service = Desc48Ref::try_from(first(&dst)).unwrap();
        assert_eq!(service.service_type(), 1);
        assert_eq!(service.provider_name()[0], 0x01);
        assert_eq!(service.provider_name_text().unwrap().to_string(), "Провайдер");
        assert_eq!(service.service_name_text().unwrap().to_string(), "Канал");
    }

    #[test]
    fn encodes_empty_names() {
        let mut dst = Vec::new();
        Desc48 {
            service_type: 1,
            provider_name: "",
            service_name: "",
            charset: Charset::Utf8,
        }
        .encode(&mut dst)
        .unwrap();

        assert_eq!(dst, b"\x48\x03\x01\x00\x00");
    }

    #[test]
    fn encode_rejects_oversized_names() {
        let name = "n".repeat(200);
        let mut dst = Vec::new();
        let result = Desc48 {
            service_type: 1,
            provider_name: &name,
            service_name: &name,
            charset: Charset::Iso6937,
        }
        .encode(&mut dst);

        assert!(result.is_err());
        assert!(dst.is_empty());
    }
}
