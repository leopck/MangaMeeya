use thiserror::Error;

#[derive(Error, Debug)]
pub enum ArchiveError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Unsupported archive format: {0}")]
    UnsupportedFormat(String),

    #[error("Entry index {index} out of range (total: {total})")]
    IndexOutOfRange { index: usize, total: usize },

    #[error("Empty archive: no image files found")]
    Empty,

    #[error("Password-protected archive: decryption not supported")]
    PasswordProtected,

    #[error("Corrupt archive: {0}")]
    Corrupt(String),
}
