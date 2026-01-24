use std::fmt;

/// Error type for PES packetizer operations
#[derive(Debug, Clone)]
pub enum PesPacketizerError {
    BufferFull { required: usize, available: usize },
}

impl fmt::Display for PesPacketizerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PesPacketizerError::BufferFull {
                required,
                available,
            } => {
                write!(
                    f,
                    "buffer full: required {} bytes, available {} bytes",
                    required, available
                )
            }
        }
    }
}

impl std::error::Error for PesPacketizerError {}
