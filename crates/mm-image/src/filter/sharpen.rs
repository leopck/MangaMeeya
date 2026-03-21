use crate::{ColorSpace, ImageBuffer, ImageError, ImageResult};
use super::ImageFilter;

/// Unsharp mask sharpening filter.
pub struct SharpenFilter {
    pub amount: f32,
    pub radius: u32,
    pub threshold: u8,
}

impl SharpenFilter {
    pub fn new(amount: f32, radius: u32, threshold: u8) -> Self {
        Self {
            amount,
            radius,
            threshold,
        }
    }
}

impl ImageFilter for SharpenFilter {
    fn name(&self) -> &str {
        "Sharpen"
    }

    fn apply(&self, image: &ImageBuffer) -> ImageResult<ImageBuffer> {
        if self.amount == 0.0 || self.radius == 0 {
            return Ok(image.clone());
        }

        if image.color_space == ColorSpace::Grayscale {
            return sharpen_gray(image, self.amount, self.radius, self.threshold);
        }

        let bpp = image.color_space.bytes_per_pixel();
        let (w, h) = (image.width as usize, image.height as usize);
        let stride = image.stride();

        // Simple 3x3 sharpen kernel approximation (unsharp mask)
        // Blur the image first, then: sharpened = original + amount * (original - blurred)
        let blurred = box_blur(image, self.radius)?;
        let mut data = image.data.clone();

        let channels = bpp.min(3); // Don't sharpen alpha
        for y in 0..h {
            for x in 0..w {
                let idx = y * stride + x * bpp;
                for c in 0..channels {
                    let orig = image.data[idx + c] as f32;
                    let blur = blurred.data[idx + c] as f32;
                    let diff = orig - blur;
                    if diff.abs() >= self.threshold as f32 {
                        let val = orig + self.amount * diff;
                        data[idx + c] = val.round().clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }

        Ok(ImageBuffer::new(data, image.width, image.height, image.color_space))
    }
}

fn sharpen_gray(image: &ImageBuffer, amount: f32, radius: u32, threshold: u8) -> ImageResult<ImageBuffer> {
    let (w, h) = (image.width as usize, image.height as usize);
    let blurred = box_blur(image, radius)?;
    let mut data = image.data.clone();

    for i in 0..w * h {
        let orig = image.data[i] as f32;
        let blur = blurred.data[i] as f32;
        let diff = orig - blur;
        if diff.abs() >= threshold as f32 {
            let val = orig + amount * diff;
            data[i] = val.round().clamp(0.0, 255.0) as u8;
        }
    }

    Ok(ImageBuffer::new(data, image.width, image.height, image.color_space))
}

/// Simple box blur for the unsharp mask.
fn box_blur(image: &ImageBuffer, radius: u32) -> ImageResult<ImageBuffer> {
    let bpp = image.color_space.bytes_per_pixel();
    let (w, h) = (image.width as i32, image.height as i32);
    let r = radius as i32;
    let kernel_size = (2 * r + 1) as f32;
    let stride = image.stride();
    let mut data = vec![0u8; image.data.len()];

    // Horizontal pass
    let mut temp = vec![0u8; image.data.len()];
    for y in 0..h {
        for x in 0..w {
            for c in 0..bpp {
                let mut sum = 0.0f32;
                for dx in -r..=r {
                    let sx = (x + dx).clamp(0, w - 1) as usize;
                    sum += image.data[y as usize * stride + sx * bpp + c] as f32;
                }
                temp[y as usize * stride + x as usize * bpp + c] =
                    (sum / kernel_size).round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    // Vertical pass
    for y in 0..h {
        for x in 0..w {
            for c in 0..bpp {
                let mut sum = 0.0f32;
                for dy in -r..=r {
                    let sy = (y + dy).clamp(0, h - 1) as usize;
                    sum += temp[sy * stride + x as usize * bpp + c] as f32;
                }
                data[y as usize * stride + x as usize * bpp + c] =
                    (sum / kernel_size).round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    Ok(ImageBuffer::new(data, image.width as u32, image.height as u32, image.color_space))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sharpen_basic() {
        let mut img = ImageBuffer::blank(20, 20, ColorSpace::Rgb8);
        // Create some contrast
        for y in 0..20u32 {
            for x in 0..20u32 {
                let v = if (x + y) % 2 == 0 { 200u8 } else { 50u8 };
                img.set_pixel(x, y, &[v, v, v]);
            }
        }
        let filter = SharpenFilter::new(1.0, 1, 0);
        let result = filter.apply(&img).unwrap();
        assert_ne!(result.data, img.data);
    }

    #[test]
    fn test_sharpen_zero_amount() {
        let img = ImageBuffer::blank(10, 10, ColorSpace::Rgb8);
        let filter = SharpenFilter::new(0.0, 1, 0);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.data, img.data);
    }

    #[test]
    fn test_sharpen_zero_radius() {
        let img = ImageBuffer::blank(10, 10, ColorSpace::Rgb8);
        let filter = SharpenFilter::new(1.0, 0, 0);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.data, img.data);
    }

    #[test]
    fn test_sharpen_uniform_image() {
        // A uniform image shouldn't change (no edges to sharpen)
        let data = vec![128u8; 10 * 10 * 3];
        let img = ImageBuffer::new(data, 10, 10, ColorSpace::Rgb8);
        let filter = SharpenFilter::new(2.0, 1, 0);
        let result = filter.apply(&img).unwrap();
        // Should be very close to original (box blur of uniform = uniform)
        for (a, b) in result.data.iter().zip(img.data.iter()) {
            assert!((*a as i32 - *b as i32).abs() <= 1);
        }
    }

    #[test]
    fn test_sharpen_grayscale() {
        let data = vec![128u8; 10 * 10];
        let img = ImageBuffer::new(data, 10, 10, ColorSpace::Grayscale);
        let filter = SharpenFilter::new(1.0, 1, 0);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.color_space, ColorSpace::Grayscale);
    }
}
