#[derive(Debug)]
pub enum PsiSectionError {
    InvalidSectionLength,
    InvalidDescriptorLength,
    InvalidTableId,
    InvalidCrc32,
}

impl core::error::Error for PsiSectionError {}

impl std::fmt::Display for PsiSectionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PsiSectionError::InvalidSectionLength => "Invalid section length".fmt(f),
            PsiSectionError::InvalidDescriptorLength => "Invalid descriptor length".fmt(f),
            PsiSectionError::InvalidTableId => "Invalid table_id".fmt(f),
            PsiSectionError::InvalidCrc32 => "Invalid CRC32".fmt(f),
        }
    }
}
