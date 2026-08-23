use crate::{
    pack_bits,
    psi::{
        Descriptor,
        DescriptorRef,
        PsiSectionError,
    },
};

/// terrestrial_delivery_system_descriptor (tag `0x5a`): tuning parameters of
/// a DVB-T multiplex.
#[derive(Debug, Clone, Copy)]
pub struct Desc5ARef<'a>(&'a [u8]);

impl<'a> Desc5ARef<'a> {
    /// Descriptor tag.
    pub const TAG: u8 = 0x5a;

    /// Centre frequency in Hz (10 Hz resolution on the wire).
    pub fn centre_frequency(&self) -> u64 {
        u64::from(u32::from_be_bytes([self.0[0], self.0[1], self.0[2], self.0[3]])) * 10
    }

    /// Bandwidth:
    /// * `0b000` - 8 MHz
    /// * `0b001` - 7 MHz
    /// * `0b010` - 6 MHz
    /// * `0b011` - 5 MHz
    pub fn bandwidth(&self) -> u8 {
        (self.0[4] & 0xe0) >> 5
    }

    /// `true` for the high-priority stream; set for non-hierarchical
    /// transmission.
    pub fn priority(&self) -> bool {
        (self.0[4] & 0x10) != 0
    }

    /// Set when time slicing is not used.
    pub fn time_slicing_indicator(&self) -> bool {
        (self.0[4] & 0x08) != 0
    }

    /// Set when MPE-FEC is not used.
    pub fn mpe_fec_indicator(&self) -> bool {
        (self.0[4] & 0x04) != 0
    }

    /// Constellation:
    /// * `0b00` - QPSK
    /// * `0b01` - 16-QAM
    /// * `0b10` - 64-QAM
    pub fn constellation(&self) -> u8 {
        (self.0[5] & 0xc0) >> 6
    }

    /// Hierarchy information (alpha and interleaver selection).
    pub fn hierarchy_information(&self) -> u8 {
        (self.0[5] & 0x38) >> 3
    }

    /// Code rate of the high-priority stream:
    /// * `0b000`-`0b100` - 1/2, 2/3, 3/4, 5/6, 7/8
    pub fn code_rate_hp_stream(&self) -> u8 {
        self.0[5] & 0x07
    }

    /// Code rate of the low-priority stream; meaningful for hierarchical
    /// transmission only.
    pub fn code_rate_lp_stream(&self) -> u8 {
        (self.0[6] & 0xe0) >> 5
    }

    /// Guard interval:
    /// * `0b00` - 1/32
    /// * `0b01` - 1/16
    /// * `0b10` - 1/8
    /// * `0b11` - 1/4
    pub fn guard_interval(&self) -> u8 {
        (self.0[6] & 0x18) >> 3
    }

    /// Transmission mode:
    /// * `0b00` - 2k
    /// * `0b01` - 8k
    /// * `0b10` - 4k
    pub fn transmission_mode(&self) -> u8 {
        (self.0[6] & 0x06) >> 1
    }

    /// Set when other frequencies are in use.
    pub fn other_frequency_flag(&self) -> bool {
        (self.0[6] & 0x01) != 0
    }
}

impl<'a> TryFrom<DescriptorRef<'a>> for Desc5ARef<'a> {
    type Error = PsiSectionError;

    fn try_from(descriptor: DescriptorRef<'a>) -> Result<Self, Self::Error> {
        if descriptor.tag() != Self::TAG {
            return Err(PsiSectionError::InvalidDescriptorTag);
        }
        let data = descriptor.data();
        if data.len() != 11 {
            return Err(PsiSectionError::InvalidDescriptorLength);
        }
        Ok(Desc5ARef(data))
    }
}

/// terrestrial_delivery_system_descriptor (tag `0x5a`) encoder.
#[derive(Debug, Clone, Copy)]
pub struct Desc5A {
    /// Centre frequency in Hz, truncated to 10 Hz resolution on the wire
    pub centre_frequency: u64,
    pub bandwidth: u8,
    /// `true` for the high-priority stream; set for non-hierarchical
    /// transmission
    pub priority: bool,
    /// Set when time slicing is not used
    pub time_slicing_indicator: bool,
    /// Set when MPE-FEC is not used
    pub mpe_fec_indicator: bool,
    pub constellation: u8,
    pub hierarchy_information: u8,
    pub code_rate_hp_stream: u8,
    /// Meaningful for hierarchical transmission only
    pub code_rate_lp_stream: u8,
    pub guard_interval: u8,
    pub transmission_mode: u8,
    pub other_frequency_flag: bool,
}

impl Descriptor for Desc5A {
    fn encode(&self, dst: &mut Vec<u8>) -> Result<(), PsiSectionError> {
        // 32-bit field of 10 Hz units
        debug_assert!(self.centre_frequency / 10 <= u64::from(u32::MAX));

        dst.push(Desc5ARef::TAG);
        dst.push(11);
        dst.extend_from_slice(&((self.centre_frequency / 10) as u32).to_be_bytes());
        dst.extend_from_slice(&pack_bits!(u8,
            bandwidth: 3 => self.bandwidth,
            priority: 1 => self.priority,
            time_slicing_indicator: 1 => self.time_slicing_indicator,
            mpe_fec_indicator: 1 => self.mpe_fec_indicator,
            reserved: 2 => 0b11,
        ));
        dst.extend_from_slice(&pack_bits!(u16,
            constellation: 2 => self.constellation,
            hierarchy_information: 3 => self.hierarchy_information,
            code_rate_hp_stream: 3 => self.code_rate_hp_stream,
            code_rate_lp_stream: 3 => self.code_rate_lp_stream,
            guard_interval: 2 => self.guard_interval,
            transmission_mode: 2 => self.transmission_mode,
            other_frequency_flag: 1 => self.other_frequency_flag,
        ));
        dst.extend_from_slice(&[0xff; 4]); // reserved_future_use
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
    fn encodes_terrestrial_delivery() {
        let mut dst = Vec::new();
        Desc5A {
            centre_frequency: 474_000_000,
            bandwidth: 0,
            priority: true,
            time_slicing_indicator: true,
            mpe_fec_indicator: true,
            constellation: 2,
            hierarchy_information: 0,
            code_rate_hp_stream: 1,
            code_rate_lp_stream: 0,
            guard_interval: 3,
            transmission_mode: 1,
            other_frequency_flag: false,
        }
        .encode(&mut dst)
        .unwrap();

        assert_eq!(
            dst,
            [
                0x5a, 0x0b, 0x02, 0xd3, 0x44, 0x40, 0x1f, 0x81, 0x1a, 0xff, 0xff, 0xff, 0xff
            ]
        );
    }

    #[test]
    fn roundtrips_terrestrial_delivery() {
        let mut dst = Vec::new();
        Desc5A {
            centre_frequency: 682_000_000,
            bandwidth: 1,
            priority: false,
            time_slicing_indicator: true,
            mpe_fec_indicator: false,
            constellation: 1,
            hierarchy_information: 2,
            code_rate_hp_stream: 4,
            code_rate_lp_stream: 3,
            guard_interval: 0,
            transmission_mode: 2,
            other_frequency_flag: true,
        }
        .encode(&mut dst)
        .unwrap();

        let terr = Desc5ARef::try_from(first(&dst)).unwrap();
        assert_eq!(terr.centre_frequency(), 682_000_000);
        assert_eq!(terr.bandwidth(), 1);
        assert!(!terr.priority());
        assert!(terr.time_slicing_indicator());
        assert!(!terr.mpe_fec_indicator());
        assert_eq!(terr.constellation(), 1);
        assert_eq!(terr.hierarchy_information(), 2);
        assert_eq!(terr.code_rate_hp_stream(), 4);
        assert_eq!(terr.code_rate_lp_stream(), 3);
        assert_eq!(terr.guard_interval(), 0);
        assert_eq!(terr.transmission_mode(), 2);
        assert!(terr.other_frequency_flag());
    }

    #[test]
    fn rejects_wrong_tag() {
        let bytes = [0x43, 0x0b, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        assert!(Desc5ARef::try_from(first(&bytes)).is_err());
    }

    #[test]
    fn rejects_wrong_length() {
        let bytes = [0x5a, 0x04, 0, 0, 0, 0];
        assert!(Desc5ARef::try_from(first(&bytes)).is_err());
    }
}
