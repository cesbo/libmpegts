#[derive(Debug)]
pub enum PsiSectionError {
    InvalidLength,
    InvalidTableId,
    InvalidCrc32,
}

impl core::error::Error for PsiSectionError {}

impl std::fmt::Display for PsiSectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsiSectionError::InvalidLength => write!(f, "Invalid section length"),
            PsiSectionError::InvalidTableId => write!(f, "Invalid table_id"),
            PsiSectionError::InvalidCrc32 => write!(f, "Invalid CRC32"),
        }
    }
}
