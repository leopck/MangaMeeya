pub mod buffer;
pub mod codec;
pub mod error;
pub mod filter;

pub use buffer::{ColorSpace, ImageBuffer};
pub use error::ImageError;

pub type ImageResult<T> = Result<T, ImageError>;
