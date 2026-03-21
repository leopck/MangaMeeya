use thiserror::Error;

#[derive(Error, Debug)]
pub enum ImageError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image decode error: {0}")]
    Decode(String),

    #[error("Unsupported image format: {0}")]
    UnsupportedFormat(String),

    #[error("Invalid dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    #[error("Filter error: {0}")]
    Filter(String),

    #[error("Image too large: {width}x{height} exceeds maximum")]
    TooLarge { width: u32, height: u32 },
}

impl From<image::ImageError> for ImageError {
    fn from(e: image::ImageError) -> Self {
        ImageError::Decode(e.to_string())
    }
}
