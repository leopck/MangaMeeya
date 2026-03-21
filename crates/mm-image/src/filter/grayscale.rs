use crate::{ColorSpace, ImageBuffer, ImageError, ImageResult};
use super::ImageFilter;

pub struct GrayscaleFilter;

impl ImageFilter for GrayscaleFilter {
    fn name(&self) -> &str {
        "Grayscale"
    }

    fn apply(&self, image: &ImageBuffer) -> ImageResult<ImageBuffer> {
        let bpp = image.color_space.bytes_per_pixel();
        if bpp < 3 {
            // Already grayscale
            return Ok(image.clone());
        }

        let pixel_count = image.width as usize * image.height as usize;
        let mut data = vec![0u8; pixel_count];

        for i in 0..pixel_count {
            let src = i * bpp;
            let r = image.data[src] as f32;
            let g = image.data[src + 1] as f32;
            let b = image.data[src + 2] as f32;
            // ITU-R BT.601 luma coefficients
            data[i] = (0.299 * r + 0.587 * g + 0.114 * b).round().clamp(0.0, 255.0) as u8;
        }

        Ok(ImageBuffer::new(
            data,
            image.width,
            image.height,
            ColorSpace::Grayscale,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grayscale_rgb() {
        let mut img = ImageBuffer::blank(3, 3, ColorSpace::Rgb8);
        img.set_pixel(0, 0, &[255, 255, 255]);
        img.set_pixel(1, 0, &[0, 0, 0]);
        let result = GrayscaleFilter.apply(&img).unwrap();
        assert_eq!(result.color_space, ColorSpace::Grayscale);
        assert_eq!(result.pixel(0, 0), &[255]); // white -> 255
        assert_eq!(result.pixel(1, 0), &[0]);   // black -> 0
    }

    #[test]
    fn test_grayscale_rgba() {
        let img = ImageBuffer::blank(5, 5, ColorSpace::Rgba8);
        let result = GrayscaleFilter.apply(&img).unwrap();
        assert_eq!(result.color_space, ColorSpace::Grayscale);
        assert_eq!(result.size_bytes(), 25);
    }

    #[test]
    fn test_grayscale_already_gray() {
        let img = ImageBuffer::blank(5, 5, ColorSpace::Grayscale);
        let result = GrayscaleFilter.apply(&img).unwrap();
        assert_eq!(result.data, img.data);
    }

    #[test]
    fn test_grayscale_all_r_g_b_equal() {
        let result_img = GrayscaleFilter
            .apply(&ImageBuffer::blank(10, 10, ColorSpace::Rgb8))
            .unwrap();
        // All black -> all gray values should be 0
        assert!(result_img.data.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_grayscale_pure_red() {
        let mut img = ImageBuffer::blank(1, 1, ColorSpace::Rgb8);
        img.set_pixel(0, 0, &[255, 0, 0]);
        let result = GrayscaleFilter.apply(&img).unwrap();
        // 0.299 * 255 = 76.245 -> 76
        assert_eq!(result.pixel(0, 0), &[76]);
    }
}
