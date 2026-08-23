use crate::{
    psi::{
        Descriptor,
        DescriptorRef,
        PsiSectionError,
    },
    utils::Bcd,
};

/// cable_delivery_system_descriptor (tag `0x44`): tuning parameters of a
/// DVB-C multiplex.
#[derive(Debug, Clone, Copy)]
pub struct CableDeliveryDescriptorRef<'a>(&'a [u8]);

impl<'a> CableDeliveryDescriptorRef<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x44;

    /// Frequency in Hz (100 Hz resolution on the wire).
    pub fn frequency(&self) -> u64 {
        u64::from(u32::from_bcd([self.0[0], self.0[1], self.0[2], self.0[3]])) * 100
    }

    /// Outer FEC scheme:
    /// * `0b0000` - undefined
    /// * `0b0001` - no outer FEC
    /// * `0b0010` - RS(204/188)
    pub fn fec_outer(&self) -> u8 {
        self.0[5] & 0x0f
    }

    /// Modulation:
    /// * `0x00` - undefined
    /// * `0x01`-`0x05` - 16-QAM, 32-QAM, 64-QAM, 128-QAM, 256-QAM
    pub fn modulation(&self) -> u8 {
        self.0[6]
    }

    /// Symbol rate in symbols per second (100 symbol/s resolution on the wire).
    pub fn symbol_rate(&self) -> u32 {
        let raw = u32::from_be_bytes([self.0[7], self.0[8], self.0[9], self.0[10]]) >> 4;
        u32::from_bcd(raw.to_be_bytes()) * 100
    }

    /// Inner FEC scheme:
    /// * `0b0000` - undefined
    /// * `0b0001`-`0b1001` - 1/2, 2/3, 3/4, 5/6, 7/8, 8/9, 3/5, 4/5, 9/10
    /// * `0b1111` - no inner FEC
    pub fn fec_inner(&self) -> u8 {
        self.0[10] & 0x0f
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for CableDeliveryDescriptorRef<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        if data.len() != 11 {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(CableDeliveryDescriptorRef(data))
    }
}

/// cable_delivery_system_descriptor (tag `0x44`) encoder.
#[derive(Debug, Clone, Copy)]
pub struct CableDeliveryDescriptor {
    /// Frequency in Hz, truncated to 100 Hz resolution on the wire
    pub frequency: u64,
    pub fec_outer: u8,
    pub modulation: u8,
    /// Symbol rate in symbols per second, truncated to 100 symbol/s resolution
    pub symbol_rate: u32,
    pub fec_inner: u8,
}

impl Descriptor for CableDeliveryDescriptor {
    fn encode(&self, dst: &mut Vec<u8>) -> Result<(), PsiSectionError> {
        // 8 BCD digits of 100 Hz and 7 BCD digits of 100 symbol/s
        debug_assert!(self.frequency < 10_000_000_000);
        debug_assert!(self.symbol_rate < 1_000_000_000);

        dst.push(CableDeliveryDescriptorRef::TAG);
        dst.push(11);
        dst.extend_from_slice(&((self.frequency / 100) as u32).into_bcd());
        dst.push(0xff); // reserved_future_use
        dst.push(0xf0 | (self.fec_outer & 0x0f));
        dst.push(self.modulation);
        let symbol_rate = u32::from_be_bytes((self.symbol_rate / 100).into_bcd());
        dst.extend_from_slice(&((symbol_rate << 4) | u32::from(self.fec_inner & 0x0f)).to_be_bytes());
        Ok(())
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
    fn encodes_cable_delivery() {
        let mut dst = Vec::new();
        CableDeliveryDescriptor {
            frequency: 346_000_000,
            fec_outer: 2,
            modulation: 0x03,
            symbol_rate: 6_875_000,
            fec_inner: 0x0f,
        }
        .encode(&mut dst)
        .unwrap();

        assert_eq!(
            dst,
            [
                0x44, 0x0b, 0x03, 0x46, 0x00, 0x00, 0xff, 0xf2, 0x03, 0x00, 0x68, 0x75, 0x0f
            ]
        );
    }

    #[test]
    fn roundtrips_cable_delivery() {
        let mut dst = Vec::new();
        CableDeliveryDescriptor {
            frequency: 858_000_000,
            fec_outer: 1,
            modulation: 0x05,
            symbol_rate: 6_900_000,
            fec_inner: 0,
        }
        .encode(&mut dst)
        .unwrap();

        let cable = CableDeliveryDescriptorRef::try_from(first(&dst)).unwrap();
        assert_eq!(cable.frequency(), 858_000_000);
        assert_eq!(cable.fec_outer(), 1);
        assert_eq!(cable.modulation(), 0x05);
        assert_eq!(cable.symbol_rate(), 6_900_000);
        assert_eq!(cable.fec_inner(), 0);
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = [0x43, 0x0b, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(CableDeliveryDescriptorRef::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_wrong_length() {
        let bytes = [0x44, 0x04, 0, 0, 0, 0];
        assert!(CableDeliveryDescriptorRef::try_from(first(&bytes)).is_err());
    }
}
