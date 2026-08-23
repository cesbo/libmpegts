use crate::{
    pack_bits,
    psi::{
        Descriptor,
        DescriptorRef,
        PsiSectionError,
    },
    utils::Bcd,
};

/// satellite_delivery_system_descriptor (tag `0x43`): tuning parameters of a
/// DVB-S/S2 transponder.
#[derive(Debug, Clone, Copy)]
pub struct Desc43Ref<'a>(&'a [u8]);

impl<'a> Desc43Ref<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x43;

    /// Frequency in kHz (10 kHz resolution on the wire).
    pub fn frequency(&self) -> u32 {
        u32::from_bcd([self.0[0], self.0[1], self.0[2], self.0[3]]) * 10
    }

    /// Orbital position in tenths of a degree.
    pub fn orbital_position(&self) -> u16 {
        u16::from_bcd([self.0[4], self.0[5]])
    }

    /// `true` for eastern position, `false` for western.
    pub fn west_east_flag(&self) -> bool {
        (self.0[6] & 0x80) != 0
    }

    /// Polarization:
    /// * `0b00` - linear horizontal
    /// * `0b01` - linear vertical
    /// * `0b10` - circular left
    /// * `0b11` - circular right
    pub fn polarization(&self) -> u8 {
        (self.0[6] & 0x60) >> 5
    }

    /// Roll-off factor, meaningful for DVB-S2 only:
    /// * `0b00` - 0.35
    /// * `0b01` - 0.25
    /// * `0b10` - 0.20
    pub fn roll_off(&self) -> u8 {
        (self.0[6] & 0x18) >> 3
    }

    /// `true` for DVB-S2, `false` for DVB-S.
    pub fn modulation_system(&self) -> bool {
        (self.0[6] & 0x04) != 0
    }

    /// Modulation type:
    /// * `0b00` - auto
    /// * `0b01` - QPSK
    /// * `0b10` - 8PSK
    /// * `0b11` - 16-QAM
    pub fn modulation_type(&self) -> u8 {
        self.0[6] & 0x03
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

impl<'a> TryFrom<DescriptorRef<'a>> for Desc43Ref<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        if data.len() != 11 {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(Desc43Ref(data))
    }
}

/// satellite_delivery_system_descriptor (tag `0x43`) encoder.
#[derive(Debug, Clone, Copy)]
pub struct Desc43 {
    /// Frequency in kHz, truncated to 10 kHz resolution on the wire
    pub frequency: u32,
    /// Orbital position in tenths of a degree
    pub orbital_position: u16,
    /// `true` for eastern position
    pub west_east_flag: bool,
    pub polarization: u8,
    /// Meaningful for DVB-S2 only, `0b00` for DVB-S
    pub roll_off: u8,
    /// `true` for DVB-S2
    pub modulation_system: bool,
    pub modulation_type: u8,
    /// Symbol rate in symbols per second, truncated to 100 symbol/s resolution
    pub symbol_rate: u32,
    pub fec_inner: u8,
}

impl Descriptor for Desc43 {
    fn encode(&self, dst: &mut Vec<u8>) -> Result<(), PsiSectionError> {
        // 8 BCD digits of 10 kHz and 7 BCD digits of 100 symbol/s
        debug_assert!(self.frequency < 1_000_000_000);
        debug_assert!(self.symbol_rate < 1_000_000_000);

        dst.push(Desc43Ref::TAG);
        dst.push(11);
        dst.extend_from_slice(&(self.frequency / 10).into_bcd());
        dst.extend_from_slice(&self.orbital_position.into_bcd());
        dst.extend_from_slice(&pack_bits!(u8,
            west_east_flag: 1 => self.west_east_flag,
            polarization: 2 => self.polarization,
            roll_off: 2 => self.roll_off,
            modulation_system: 1 => self.modulation_system,
            modulation_type: 2 => self.modulation_type,
        ));
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
    fn encodes_satellite_delivery() {
        let mut dst = Vec::new();
        Desc43 {
            frequency: 11_727_480,
            orbital_position: 130,
            west_east_flag: true,
            polarization: 0,
            roll_off: 0,
            modulation_system: true,
            modulation_type: 1,
            symbol_rate: 27_500_000,
            fec_inner: 3,
        }
        .encode(&mut dst)
        .unwrap();

        assert_eq!(
            dst,
            [
                0x43, 0x0b, 0x01, 0x17, 0x27, 0x48, 0x01, 0x30, 0x85, 0x02, 0x75, 0x00, 0x03
            ]
        );
    }

    #[test]
    fn roundtrips_satellite_delivery() {
        let mut dst = Vec::new();
        Desc43 {
            frequency: 12_640_000,
            orbital_position: 425,
            west_east_flag: false,
            polarization: 1,
            roll_off: 2,
            modulation_system: false,
            modulation_type: 2,
            symbol_rate: 30_000_000,
            fec_inner: 5,
        }
        .encode(&mut dst)
        .unwrap();

        let sat = Desc43Ref::try_from(first(&dst)).unwrap();
        assert_eq!(sat.frequency(), 12_640_000);
        assert_eq!(sat.orbital_position(), 425);
        assert!(!sat.west_east_flag());
        assert_eq!(sat.polarization(), 1);
        assert_eq!(sat.roll_off(), 2);
        assert!(!sat.modulation_system());
        assert_eq!(sat.modulation_type(), 2);
        assert_eq!(sat.symbol_rate(), 30_000_000);
        assert_eq!(sat.fec_inner(), 5);
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = [0x44, 0x0b, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(Desc43Ref::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_wrong_length() {
        let bytes = [0x43, 0x04, 0, 0, 0, 0];
        assert!(Desc43Ref::try_from(first(&bytes)).is_err());
    }
}
