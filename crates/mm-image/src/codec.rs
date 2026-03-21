use crate::{ColorSpace, ImageBuffer, ImageError, ImageResult};

/// Detect image format from magic bytes in the data.
pub fn detect_format(data: &[u8]) -> Option<ImageFormat> {
    if data.len() < 4 {
        return None;
    }

    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(ImageFormat::Jpeg)
    } else if data.starts_with(&[0x89, 0x50, 0x4E, 0x47]) {
        Some(ImageFormat::Png)
    } else if data.starts_with(b"RIFF") && data.len() >= 12 && &data[8..12] == b"WEBP" {
        Some(ImageFormat::WebP)
    } else if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        Some(ImageFormat::Gif)
    } else if data.starts_with(&[0x42, 0x4D]) {
        Some(ImageFormat::Bmp)
    } else if data.len() >= 12 && &data[4..8] == b"ftyp" {
        Some(ImageFormat::Avif)
    } else if data.starts_with(&[0x49, 0x49, 0x2A, 0x00])
        || data.starts_with(&[0x4D, 0x4D, 0x00, 0x2A])
    {
        Some(ImageFormat::Tiff)
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Jpeg,
    Png,
    WebP,
    Gif,
    Bmp,
    Avif,
    Jxl,
    Tiff,
    Jp2,
    Psd,
}

/// Decode raw image bytes into an ImageBuffer.
/// Uses magic-byte detection to determine the format, falling back to the `image` crate.
pub fn decode(data: &[u8]) -> ImageResult<ImageBuffer> {
    if data.is_empty() {
        return Err(ImageError::Decode("Empty data".into()));
    }

    let format = detect_format(data);

    // Use the `image` crate as a universal fallback decoder.
    let dynamic = image::load_from_memory(data)?;
    let rgba = dynamic.to_rgba8();
    let (w, h) = rgba.dimensions();

    if w == 0 || h == 0 {
        return Err(ImageError::InvalidDimensions {
            width: w,
            height: h,
        });
    }

    // Convert to RGB8 if no alpha channel is meaningful (all alpha = 255).
    let all_opaque = rgba.pixels().all(|p| p.0[3] == 255);

    if all_opaque {
        let rgb = dynamic.to_rgb8();
        Ok(ImageBuffer::new(rgb.into_raw(), w, h, ColorSpace::Rgb8))
    } else {
        Ok(ImageBuffer::new(
            rgba.into_raw(),
            w,
            h,
            ColorSpace::Rgba8,
        ))
    }
}

/// Decode to a specific target color space.
pub fn decode_as(data: &[u8], target: ColorSpace) -> ImageResult<ImageBuffer> {
    if data.is_empty() {
        return Err(ImageError::Decode("Empty data".into()));
    }

    let dynamic = image::load_from_memory(data)?;
    let (w, h) = (dynamic.width(), dynamic.height());

    match target {
        ColorSpace::Rgb8 => {
            let rgb = dynamic.to_rgb8();
            Ok(ImageBuffer::new(rgb.into_raw(), w, h, ColorSpace::Rgb8))
        }
        ColorSpace::Rgba8 => {
            let rgba = dynamic.to_rgba8();
            Ok(ImageBuffer::new(rgba.into_raw(), w, h, ColorSpace::Rgba8))
        }
        ColorSpace::Grayscale => {
            let gray = dynamic.to_luma8();
            Ok(ImageBuffer::new(
                gray.into_raw(),
                w,
                h,
                ColorSpace::Grayscale,
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a minimal valid JPEG in memory.
    fn create_test_jpeg(width: u32, height: u32) -> Vec<u8> {
        use image::{ImageEncoder, RgbImage};
        let img = RgbImage::from_fn(width, height, |x, y| {
            image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
        });
        let mut buf = Vec::new();
        let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, 90);
        encoder
            .write_image(img.as_raw(), width, height, image::ExtendedColorType::Rgb8)
            .unwrap();
        buf
    }

    /// Create a minimal valid PNG in memory.
    fn create_test_png(width: u32, height: u32) -> Vec<u8> {
        use image::{ImageEncoder, RgbaImage};
        let img = RgbaImage::from_fn(width, height, |x, y| {
            image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255])
        });
        let mut buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        encoder
            .write_image(
                img.as_raw(),
                width,
                height,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        buf
    }

    /// Create a PNG with transparency.
    fn create_test_png_with_alpha(width: u32, height: u32) -> Vec<u8> {
        use image::{ImageEncoder, RgbaImage};
        let img = RgbaImage::from_fn(width, height, |x, _y| {
            image::Rgba([255, 0, 0, (x % 256) as u8])
        });
        let mut buf = Vec::new();
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        encoder
            .write_image(
                img.as_raw(),
                width,
                height,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        buf
    }

    #[test]
    fn test_detect_format_jpeg() {
        let data = create_test_jpeg(10, 10);
        assert_eq!(detect_format(&data), Some(ImageFormat::Jpeg));
    }

    #[test]
    fn test_detect_format_png() {
        let data = create_test_png(10, 10);
        assert_eq!(detect_format(&data), Some(ImageFormat::Png));
    }

    #[test]
    fn test_detect_format_unknown() {
        assert_eq!(detect_format(b"hello world"), None);
    }

    #[test]
    fn test_detect_format_too_short() {
        assert_eq!(detect_format(&[0xFF]), None);
    }

    #[test]
    fn test_decode_jpeg() {
        let data = create_test_jpeg(100, 50);
        let img = decode(&data).unwrap();
        assert_eq!(img.width, 100);
        assert_eq!(img.height, 50);
        assert_eq!(img.color_space, ColorSpace::Rgb8);
    }

    #[test]
    fn test_decode_png() {
        let data = create_test_png(80, 60);
        let img = decode(&data).unwrap();
        assert_eq!(img.width, 80);
        assert_eq!(img.height, 60);
        // All alpha=255, so should decode to RGB8
        assert_eq!(img.color_space, ColorSpace::Rgb8);
    }

    #[test]
    fn test_decode_png_with_alpha() {
        let data = create_test_png_with_alpha(50, 50);
        let img = decode(&data).unwrap();
        assert_eq!(img.color_space, ColorSpace::Rgba8);
    }

    #[test]
    fn test_decode_empty() {
        let result = decode(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_corrupt() {
        let result = decode(b"not an image at all");
        assert!(result.is_err());
    }

    #[test]
    fn test_decode_wrong_extension_but_valid() {
        // JPEG data should decode regardless of what we call the "file"
        let data = create_test_jpeg(10, 10);
        let img = decode(&data).unwrap();
        assert_eq!(img.width, 10);
    }

    #[test]
    fn test_decode_1x1() {
        let data = create_test_jpeg(1, 1);
        let img = decode(&data).unwrap();
        assert_eq!(img.width, 1);
        assert_eq!(img.height, 1);
    }

    #[test]
    fn test_decode_as_grayscale() {
        let data = create_test_jpeg(20, 20);
        let img = decode_as(&data, ColorSpace::Grayscale).unwrap();
        assert_eq!(img.color_space, ColorSpace::Grayscale);
        assert_eq!(img.size_bytes(), 20 * 20);
    }

    #[test]
    fn test_decode_as_rgba() {
        let data = create_test_jpeg(20, 20);
        let img = decode_as(&data, ColorSpace::Rgba8).unwrap();
        assert_eq!(img.color_space, ColorSpace::Rgba8);
        assert_eq!(img.size_bytes(), 20 * 20 * 4);
    }

    #[test]
    fn test_decode_large_resolution() {
        // 2000x2000 should be fine
        let data = create_test_jpeg(2000, 2000);
        let img = decode(&data).unwrap();
        assert_eq!(img.width, 2000);
        assert_eq!(img.height, 2000);
    }
}
