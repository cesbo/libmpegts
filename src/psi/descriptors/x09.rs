use crate::{
    pack_bits,
    psi::{
        Descriptor,
        DescriptorRef,
        PsiSectionError,
    },
    ts::PID_NONE,
};

/// CA_descriptor (tag `0x09`): conditional access system identifier, CA PID and
/// optional private data bytes.
#[derive(Debug, Clone, Copy)]
pub struct Desc09Ref<'a>(&'a [u8]);

impl<'a> Desc09Ref<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x09;

    /// Conditional access system identifier.
    pub fn ca_system_id(&self) -> u16 {
        u16::from_be_bytes([self.0[0], self.0[1]])
    }

    /// PID carrying conditional access information.
    pub fn ca_pid(&self) -> u16 {
        ((u16::from(self.0[2] & 0x1f)) << 8) | u16::from(self.0[3])
    }

    /// Descriptor-private bytes following the fixed CA fields.
    pub fn private_data(&self) -> &'a [u8] {
        &self.0[4 ..]
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for Desc09Ref<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        if data.len() < 4 {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(Desc09Ref(data))
    }
}

/// CA_descriptor (tag `0x09`) encoder.
#[derive(Debug, Clone, Copy)]
pub struct Desc09<'a> {
    pub ca_system_id: u16,
    pub ca_pid: u16,
    pub private_data: &'a [u8],
}

impl Descriptor for Desc09<'_> {
    fn encode(&self, dst: &mut Vec<u8>) -> Result<(), PsiSectionError> {
        debug_assert!(self.ca_pid < PID_NONE);

        let data_len = 4 + self.private_data.len();
        if data_len > 0xff {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }

        dst.push(Desc09Ref::TAG);
        dst.push(data_len as u8);
        dst.extend_from_slice(&self.ca_system_id.to_be_bytes());
        dst.extend_from_slice(&pack_bits!(u16,
            reserved: 3 => 0b111,
            pid: 13 => self.ca_pid,
        ));
        dst.extend_from_slice(self.private_data);
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
    fn parses_ca_fields() {
        let bytes = descriptor(0x09, &[0x09, 0x63, 0xe5, 0x01, 0xaa, 0xbb]);

        let ca = Desc09Ref::try_from(first(&bytes)).unwrap();
        assert_eq!(ca.ca_system_id(), 0x0963);
        assert_eq!(ca.ca_pid(), 0x0501);
        assert_eq!(ca.private_data(), &[0xaa, 0xbb]);
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = descriptor(0x0a, &[0x09, 0x63, 0xe5, 0x01]);
        assert!(Desc09Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_short_payload() {
        let bytes = descriptor(0x09, &[0x09, 0x63, 0xe5]);
        assert!(Desc09Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn encodes_ca_fields() {
        let mut dst = Vec::new();
        Desc09 {
            ca_system_id: 0x0963,
            ca_pid: 0x0501,
            private_data: &[0xaa, 0xbb],
        }
        .encode(&mut dst)
        .unwrap();

        assert_eq!(dst, [0x09, 0x06, 0x09, 0x63, 0xe5, 0x01, 0xaa, 0xbb]);

        let ca = Desc09Ref::try_from(first(&dst)).unwrap();
        assert_eq!(ca.ca_system_id(), 0x0963);
        assert_eq!(ca.ca_pid(), 0x0501);
        assert_eq!(ca.private_data(), &[0xaa, 0xbb]);
    }

    #[test]
    fn encode_rejects_oversized_private_data() {
        let mut dst = Vec::new();
        let result = Desc09 {
            ca_system_id: 0x0963,
            ca_pid: 0x0501,
            private_data: &[0; 252],
        }
        .encode(&mut dst);

        assert!(result.is_err());
        assert!(dst.is_empty());
    }
}
