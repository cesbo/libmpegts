use textcode;


/// service_descriptor - provides the names of the service provider 
/// and the service in text form together with the service_type.
#[derive(Debug, Default)]
pub struct Desc48 {
    /// Type of the service.
    pub type_: u8,
    /// Name of the service provider.
    pub provider: textcode::StringDVB,
    /// Name of the service.
    pub name: textcode::StringDVB,
}

impl Desc48 {
    #[inline]
    pub fn min_size() -> usize {
        5
    }

    pub fn check(slice: &[u8]) -> bool {
        true
    }

    pub fn parse(slice: &[u8]) -> Self {
        let skip = 4;
        let provider_length = slice[3] as usize;

        Self {
            type_: slice[2],
            // TODO: handle empty provider
            provider: textcode::StringDVB::from(
                &slice[skip .. skip + provider_length]
            ),
            // TODO: handle empty name
            name: textcode::StringDVB::from(
                &slice[skip + provider_length + 1 ..]
            )
        }
    }

    pub fn assemble(&self, buffer: &mut Vec<u8>) {}
}
