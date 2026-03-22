use super::ImageFilter;
use crate::{ImageBuffer, ImageError, ImageResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeAlgorithm {
    Nearest,
    Bilinear,
    Lanczos3,
    Bicubic,
    PixelAveraging,
    Halftone,
}

pub struct ResizeFilter {
    width: u32,
    height: u32,
    algorithm: ResizeAlgorithm,
}

impl ResizeFilter {
    pub fn new(width: u32, height: u32, algorithm: ResizeAlgorithm) -> Self {
        Self {
            width,
            height,
            algorithm,
        }
    }

    /// Create a resize filter that fits within the given dimensions while preserving aspect ratio.
    pub fn fit(
        max_width: u32,
        max_height: u32,
        src_width: u32,
        src_height: u32,
        algorithm: ResizeAlgorithm,
    ) -> Self {
        let scale_w = max_width as f64 / src_width as f64;
        let scale_h = max_height as f64 / src_height as f64;
        let scale = scale_w.min(scale_h);
        let new_w = ((src_width as f64 * scale) as u32).max(1);
        let new_h = ((src_height as f64 * scale) as u32).max(1);
        Self::new(new_w, new_h, algorithm)
    }
}

impl ImageFilter for ResizeFilter {
    fn name(&self) -> &str {
        "Resize"
    }

    fn apply(&self, image: &ImageBuffer) -> ImageResult<ImageBuffer> {
        if self.width == 0 || self.height == 0 {
            return Err(ImageError::InvalidDimensions {
                width: self.width,
                height: self.height,
            });
        }

        if self.width == image.width && self.height == image.height {
            return Ok(image.clone());
        }

        match self.algorithm {
            ResizeAlgorithm::Nearest => resize_nearest(image, self.width, self.height),
            ResizeAlgorithm::Bilinear => resize_bilinear(image, self.width, self.height),
            ResizeAlgorithm::Lanczos3 => resize_lanczos3(image, self.width, self.height),
            ResizeAlgorithm::Bicubic => resize_bicubic(image, self.width, self.height),
            ResizeAlgorithm::PixelAveraging => resize_pixel_averaging(image, self.width, self.height),
            ResizeAlgorithm::Halftone => resize_halftone(image, self.width, self.height),
        }
    }
}

fn resize_nearest(image: &ImageBuffer, new_w: u32, new_h: u32) -> ImageResult<ImageBuffer> {
    let bpp = image.color_space.bytes_per_pixel();
    let src_stride = image.stride();
    let dst_stride = new_w as usize * bpp;
    let mut data = vec![0u8; dst_stride * new_h as usize];

    for dy in 0..new_h as usize {
        let sy = (dy * image.height as usize) / new_h as usize;
        for dx in 0..new_w as usize {
            let sx = (dx * image.width as usize) / new_w as usize;
            let src = sy * src_stride + sx * bpp;
            let dst = dy * dst_stride + dx * bpp;
            data[dst..dst + bpp].copy_from_slice(&image.data[src..src + bpp]);
        }
    }

    Ok(ImageBuffer::new(data, new_w, new_h, image.color_space))
}

fn resize_bilinear(image: &ImageBuffer, new_w: u32, new_h: u32) -> ImageResult<ImageBuffer> {
    let bpp = image.color_space.bytes_per_pixel();
    let src_stride = image.stride();
    let dst_stride = new_w as usize * bpp;
    let mut data = vec![0u8; dst_stride * new_h as usize];

    let x_ratio = if new_w > 1 {
        (image.width as f64 - 1.0) / (new_w as f64 - 1.0)
    } else {
        0.0
    };
    let y_ratio = if new_h > 1 {
        (image.height as f64 - 1.0) / (new_h as f64 - 1.0)
    } else {
        0.0
    };

    for dy in 0..new_h as usize {
        let src_y = dy as f64 * y_ratio;
        let y0 = src_y as usize;
        let y1 = (y0 + 1).min(image.height as usize - 1);
        let y_frac = src_y - y0 as f64;

        for dx in 0..new_w as usize {
            let src_x = dx as f64 * x_ratio;
            let x0 = src_x as usize;
            let x1 = (x0 + 1).min(image.width as usize - 1);
            let x_frac = src_x - x0 as f64;

            let dst = dy * dst_stride + dx * bpp;

            for c in 0..bpp {
                let p00 = image.data[y0 * src_stride + x0 * bpp + c] as f64;
                let p10 = image.data[y0 * src_stride + x1 * bpp + c] as f64;
                let p01 = image.data[y1 * src_stride + x0 * bpp + c] as f64;
                let p11 = image.data[y1 * src_stride + x1 * bpp + c] as f64;

                let top = p00 + (p10 - p00) * x_frac;
                let bottom = p01 + (p11 - p01) * x_frac;
                let value = top + (bottom - top) * y_frac;

                data[dst + c] = value.round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    Ok(ImageBuffer::new(data, new_w, new_h, image.color_space))
}

fn resize_lanczos3(image: &ImageBuffer, new_w: u32, new_h: u32) -> ImageResult<ImageBuffer> {
    // Lanczos3 resampling using separable 2-pass approach
    // Pass 1: Horizontal resize
    let horizontal = resize_lanczos3_horizontal(image, new_w)?;
    // Pass 2: Vertical resize
    resize_lanczos3_vertical(&horizontal, new_h)
}

fn resize_lanczos3_horizontal(image: &ImageBuffer, new_w: u32) -> ImageResult<ImageBuffer> {
    let bpp = image.color_space.bytes_per_pixel();
    let src_stride = image.stride();
    let dst_stride = new_w as usize * bpp;
    let mut data = vec![0u8; dst_stride * image.height as usize];

    let ratio = image.width as f64 / new_w as f64;
    let a = 3.0; // Lanczos window size

    for dy in 0..image.height as usize {
        for dx in 0..new_w as usize {
            let src_x = (dx as f64 + 0.5) * ratio - 0.5;
            let x_start = (src_x - a + 0.5).floor() as i32;
            let x_end = (src_x + a + 0.5).ceil() as i32;

            for c in 0..bpp {
                let mut sum = 0.0;
                let mut weight_sum = 0.0;

                for sx in x_start..x_end {
                    if sx >= 0 && sx < image.width as i32 {
                        let dist = (sx as f64 - src_x).abs();
                        let weight = lanczos_kernel(dist, a);
                        let pixel_val = image.data[dy * src_stride + sx as usize * bpp + c] as f64;
                        sum += pixel_val * weight;
                        weight_sum += weight;
                    }
                }

                let value = if weight_sum > 0.0 {
                    (sum / weight_sum).round().clamp(0.0, 255.0)
                } else {
                    0.0
                };
                data[dy * dst_stride + dx * bpp + c] = value as u8;
            }
        }
    }

    Ok(ImageBuffer::new(data, new_w, image.height, image.color_space))
}

fn resize_lanczos3_vertical(image: &ImageBuffer, new_h: u32) -> ImageResult<ImageBuffer> {
    let bpp = image.color_space.bytes_per_pixel();
    let src_stride = image.stride();
    let dst_stride = image.width as usize * bpp;
    let mut data = vec![0u8; dst_stride * new_h as usize];

    let ratio = image.height as f64 / new_h as f64;
    let a = 3.0; // Lanczos window size

    for dy in 0..new_h as usize {
        let src_y = (dy as f64 + 0.5) * ratio - 0.5;
        let y_start = (src_y - a + 0.5).floor() as i32;
        let y_end = (src_y + a + 0.5).ceil() as i32;

        for dx in 0..image.width as usize {
            for c in 0..bpp {
                let mut sum = 0.0;
                let mut weight_sum = 0.0;

                for sy in y_start..y_end {
                    if sy >= 0 && sy < image.height as i32 {
                        let dist = (sy as f64 - src_y).abs();
                        let weight = lanczos_kernel(dist, a);
                        let pixel_val = image.data[sy as usize * src_stride + dx * bpp + c] as f64;
                        sum += pixel_val * weight;
                        weight_sum += weight;
                    }
                }

                let value = if weight_sum > 0.0 {
                    (sum / weight_sum).round().clamp(0.0, 255.0)
                } else {
                    0.0
                };
                data[dy * dst_stride + dx * bpp + c] = value as u8;
            }
        }
    }

    Ok(ImageBuffer::new(data, image.width, new_h, image.color_space))
}

fn lanczos_kernel(x: f64, a: f64) -> f64 {
    if x.abs() < 1e-8 {
        return 1.0;
    }
    if x.abs() >= a {
        return 0.0;
    }
    let pi_x = std::f64::consts::PI * x;
    let sinc_x = pi_x.sin() / pi_x;
    let sinc_xa = (pi_x / a).sin() / (pi_x / a);
    sinc_x * sinc_xa
}

fn resize_bicubic(image: &ImageBuffer, new_w: u32, new_h: u32) -> ImageResult<ImageBuffer> {
    // Mitchell-Netravali bicubic filter using separable 2-pass approach
    // Pass 1: Horizontal resize
    let horizontal = resize_bicubic_horizontal(image, new_w)?;
    // Pass 2: Vertical resize
    resize_bicubic_vertical(&horizontal, new_h)
}

fn resize_bicubic_horizontal(image: &ImageBuffer, new_w: u32) -> ImageResult<ImageBuffer> {
    let bpp = image.color_space.bytes_per_pixel();
    let src_stride = image.stride();
    let dst_stride = new_w as usize * bpp;
    let mut data = vec![0u8; dst_stride * image.height as usize];

    let ratio = image.width as f64 / new_w as f64;

    for dy in 0..image.height as usize {
        for dx in 0..new_w as usize {
            let src_x = (dx as f64 + 0.5) * ratio - 0.5;
            let x_start = (src_x - 2.0).floor() as i32;
            let x_end = (src_x + 2.0).ceil() as i32;

            for c in 0..bpp {
                let mut sum = 0.0;
                let mut weight_sum = 0.0;

                for sx in x_start..x_end {
                    if sx >= 0 && sx < image.width as i32 {
                        let dist = (sx as f64 - src_x).abs();
                        let weight = mitchell_netravali_kernel(dist);
                        let pixel_val = image.data[dy * src_stride + sx as usize * bpp + c] as f64;
                        sum += pixel_val * weight;
                        weight_sum += weight;
                    }
                }

                let value = if weight_sum > 0.0 {
                    (sum / weight_sum).round().clamp(0.0, 255.0)
                } else {
                    0.0
                };
                data[dy * dst_stride + dx * bpp + c] = value as u8;
            }
        }
    }

    Ok(ImageBuffer::new(data, new_w, image.height, image.color_space))
}

fn resize_bicubic_vertical(image: &ImageBuffer, new_h: u32) -> ImageResult<ImageBuffer> {
    let bpp = image.color_space.bytes_per_pixel();
    let src_stride = image.stride();
    let dst_stride = image.width as usize * bpp;
    let mut data = vec![0u8; dst_stride * new_h as usize];

    let ratio = image.height as f64 / new_h as f64;

    for dy in 0..new_h as usize {
        let src_y = (dy as f64 + 0.5) * ratio - 0.5;
        let y_start = (src_y - 2.0).floor() as i32;
        let y_end = (src_y + 2.0).ceil() as i32;

        for dx in 0..image.width as usize {
            for c in 0..bpp {
                let mut sum = 0.0;
                let mut weight_sum = 0.0;

                for sy in y_start..y_end {
                    if sy >= 0 && sy < image.height as i32 {
                        let dist = (sy as f64 - src_y).abs();
                        let weight = mitchell_netravali_kernel(dist);
                        let pixel_val = image.data[sy as usize * src_stride + dx * bpp + c] as f64;
                        sum += pixel_val * weight;
                        weight_sum += weight;
                    }
                }

                let value = if weight_sum > 0.0 {
                    (sum / weight_sum).round().clamp(0.0, 255.0)
                } else {
                    0.0
                };
                data[dy * dst_stride + dx * bpp + c] = value as u8;
            }
        }
    }

    Ok(ImageBuffer::new(data, image.width, new_h, image.color_space))
}

fn mitchell_netravali_kernel(x: f64) -> f64 {
    // Mitchell-Netravali with B=1/3, C=1/3 (recommended balanced parameters)
    let b = 1.0 / 3.0;
    let c = 1.0 / 3.0;
    let x = x.abs();

    if x < 1.0 {
        ((12.0 - 9.0 * b - 6.0 * c) * x * x * x
            + (-18.0 + 12.0 * b + 6.0 * c) * x * x
            + (6.0 - 2.0 * b)) / 6.0
    } else if x < 2.0 {
        ((-b - 6.0 * c) * x * x * x
            + (6.0 * b + 30.0 * c) * x * x
            + (-12.0 * b - 48.0 * c) * x
            + (8.0 * b + 24.0 * c)) / 6.0
    } else {
        0.0
    }
}

fn resize_pixel_averaging(image: &ImageBuffer, new_w: u32, new_h: u32) -> ImageResult<ImageBuffer> {
    // Use area-based averaging for downscaling, fallback to bilinear for upscaling
    let is_downscale = new_w < image.width && new_h < image.height;

    if !is_downscale {
        return resize_bilinear(image, new_w, new_h);
    }

    let bpp = image.color_space.bytes_per_pixel();
    let src_stride = image.stride();
    let dst_stride = new_w as usize * bpp;
    let mut data = vec![0u8; dst_stride * new_h as usize];

    let x_ratio = image.width as f64 / new_w as f64;
    let y_ratio = image.height as f64 / new_h as f64;

    for dy in 0..new_h as usize {
        let src_y_start = dy as f64 * y_ratio;
        let src_y_end = ((dy + 1) as f64 * y_ratio).min(image.height as f64);

        for dx in 0..new_w as usize {
            let src_x_start = dx as f64 * x_ratio;
            let src_x_end = ((dx + 1) as f64 * x_ratio).min(image.width as f64);

            for c in 0..bpp {
                let mut sum = 0.0;
                let mut count = 0.0;

                let y_start = src_y_start.floor() as usize;
                let y_end = src_y_end.ceil() as usize;
                let x_start = src_x_start.floor() as usize;
                let x_end = src_x_end.ceil() as usize;

                for sy in y_start..y_end.min(image.height as usize) {
                    let y_overlap = {
                        let y_low = (sy as f64).max(src_y_start);
                        let y_high = ((sy + 1) as f64).min(src_y_end);
                        y_high - y_low
                    };

                    for sx in x_start..x_end.min(image.width as usize) {
                        let x_overlap = {
                            let x_low = (sx as f64).max(src_x_start);
                            let x_high = ((sx + 1) as f64).min(src_x_end);
                            x_high - x_low
                        };

                        let weight = x_overlap * y_overlap;
                        let pixel_val = image.data[sy * src_stride + sx * bpp + c] as f64;
                        sum += pixel_val * weight;
                        count += weight;
                    }
                }

                let value = if count > 0.0 {
                    (sum / count).round().clamp(0.0, 255.0)
                } else {
                    0.0
                };
                data[dy * dst_stride + dx * bpp + c] = value as u8;
            }
        }
    }

    Ok(ImageBuffer::new(data, new_w, new_h, image.color_space))
}

fn resize_halftone(image: &ImageBuffer, new_w: u32, new_h: u32) -> ImageResult<ImageBuffer> {
    // First resize using nearest neighbor
    let resized = resize_nearest(image, new_w, new_h)?;

    // Apply ordered dithering using Bayer 4x4 matrix
    let bayer_matrix = [
        [0, 8, 2, 10],
        [12, 4, 14, 6],
        [3, 11, 1, 9],
        [15, 7, 13, 5],
    ];

    let bpp = resized.color_space.bytes_per_pixel();
    let stride = resized.stride();
    let mut data = resized.data.clone();

    for y in 0..new_h as usize {
        for x in 0..new_w as usize {
            let threshold = (bayer_matrix[y % 4][x % 4] as f64 / 16.0) * 255.0;

            for c in 0..bpp {
                let offset = y * stride + x * bpp + c;
                let pixel_val = data[offset] as f64;

                // Apply dithering threshold
                let dithered = if pixel_val > threshold {
                    255
                } else {
                    0
                };

                data[offset] = dithered;
            }
        }
    }

    Ok(ImageBuffer::new(data, new_w, new_h, resized.color_space))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ColorSpace;

    #[test]
    fn test_resize_nearest_downscale() {
        let img = ImageBuffer::blank(100, 100, ColorSpace::Rgb8);
        let filter = ResizeFilter::new(50, 50, ResizeAlgorithm::Nearest);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.width, 50);
        assert_eq!(result.height, 50);
    }

    #[test]
    fn test_resize_nearest_upscale() {
        let img = ImageBuffer::blank(50, 50, ColorSpace::Rgb8);
        let filter = ResizeFilter::new(200, 200, ResizeAlgorithm::Nearest);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.width, 200);
        assert_eq!(result.height, 200);
    }

    #[test]
    fn test_resize_bilinear_downscale() {
        let img = ImageBuffer::blank(100, 100, ColorSpace::Rgb8);
        let filter = ResizeFilter::new(50, 50, ResizeAlgorithm::Bilinear);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.width, 50);
        assert_eq!(result.height, 50);
    }

    #[test]
    fn test_resize_same_size() {
        let mut img = ImageBuffer::blank(10, 10, ColorSpace::Rgb8);
        img.set_pixel(5, 5, &[255, 128, 0]);
        let filter = ResizeFilter::new(10, 10, ResizeAlgorithm::Nearest);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.data, img.data);
    }

    #[test]
    fn test_resize_to_1x1() {
        let img = ImageBuffer::blank(100, 100, ColorSpace::Rgb8);
        let filter = ResizeFilter::new(1, 1, ResizeAlgorithm::Nearest);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.width, 1);
        assert_eq!(result.height, 1);
    }

    #[test]
    fn test_resize_zero_dims() {
        let img = ImageBuffer::blank(10, 10, ColorSpace::Rgb8);
        let filter = ResizeFilter::new(0, 10, ResizeAlgorithm::Nearest);
        assert!(filter.apply(&img).is_err());
    }

    #[test]
    fn test_resize_nearest_pixel_duplication() {
        let mut img = ImageBuffer::blank(2, 2, ColorSpace::Rgb8);
        img.set_pixel(0, 0, &[255, 0, 0]);
        img.set_pixel(1, 0, &[0, 255, 0]);
        img.set_pixel(0, 1, &[0, 0, 255]);
        img.set_pixel(1, 1, &[255, 255, 0]);
        let filter = ResizeFilter::new(4, 4, ResizeAlgorithm::Nearest);
        let result = filter.apply(&img).unwrap();
        // Each pixel should be duplicated to a 2x2 block
        assert_eq!(result.pixel(0, 0), &[255, 0, 0]);
        assert_eq!(result.pixel(1, 0), &[255, 0, 0]);
        assert_eq!(result.pixel(2, 0), &[0, 255, 0]);
        assert_eq!(result.pixel(3, 0), &[0, 255, 0]);
    }

    #[test]
    fn test_resize_fit_aspect_ratio() {
        let filter = ResizeFilter::fit(800, 600, 1000, 500, ResizeAlgorithm::Bilinear);
        // 1000x500 -> fit in 800x600 -> scale by 0.8 -> 800x400
        assert_eq!(filter.width, 800);
        assert_eq!(filter.height, 400);
    }

    #[test]
    fn test_resize_fit_tall_image() {
        let filter = ResizeFilter::fit(800, 600, 400, 1200, ResizeAlgorithm::Bilinear);
        // 400x1200 -> fit in 800x600 -> scale by 0.5 -> 200x600
        assert_eq!(filter.width, 200);
        assert_eq!(filter.height, 600);
    }

    #[test]
    fn test_resize_rgba() {
        let img = ImageBuffer::blank(20, 20, ColorSpace::Rgba8);
        let filter = ResizeFilter::new(10, 10, ResizeAlgorithm::Bilinear);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.color_space, ColorSpace::Rgba8);
        assert_eq!(result.size_bytes(), 10 * 10 * 4);
    }

    #[test]
    fn test_resize_lanczos3_downscale() {
        let img = ImageBuffer::blank(100, 100, ColorSpace::Rgb8);
        let filter = ResizeFilter::new(50, 50, ResizeAlgorithm::Lanczos3);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.width, 50);
        assert_eq!(result.height, 50);
        assert_eq!(result.color_space, ColorSpace::Rgb8);
    }

    #[test]
    fn test_resize_lanczos3_upscale() {
        let mut img = ImageBuffer::blank(10, 10, ColorSpace::Rgb8);
        img.set_pixel(5, 5, &[255, 128, 64]);
        let filter = ResizeFilter::new(20, 20, ResizeAlgorithm::Lanczos3);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.width, 20);
        assert_eq!(result.height, 20);
        // Lanczos should preserve some color information
        assert!(result.data.iter().any(|&v| v > 0));
    }

    #[test]
    fn test_resize_bicubic_downscale() {
        let img = ImageBuffer::blank(80, 60, ColorSpace::Rgb8);
        let filter = ResizeFilter::new(40, 30, ResizeAlgorithm::Bicubic);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.width, 40);
        assert_eq!(result.height, 30);
        assert_eq!(result.color_space, ColorSpace::Rgb8);
    }

    #[test]
    fn test_resize_bicubic_upscale() {
        let mut img = ImageBuffer::blank(15, 15, ColorSpace::Rgb8);
        img.set_pixel(7, 7, &[200, 100, 50]);
        let filter = ResizeFilter::new(30, 30, ResizeAlgorithm::Bicubic);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.width, 30);
        assert_eq!(result.height, 30);
        // Bicubic should smooth the interpolation
        assert!(result.data.iter().any(|&v| v > 0));
    }

    #[test]
    fn test_resize_pixel_averaging_downscale() {
        let mut img = ImageBuffer::blank(100, 100, ColorSpace::Rgb8);
        // Fill with a pattern
        for y in 0..100 {
            for x in 0..100 {
                img.set_pixel(x, y, &[128, 128, 128]);
            }
        }
        let filter = ResizeFilter::new(50, 50, ResizeAlgorithm::PixelAveraging);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.width, 50);
        assert_eq!(result.height, 50);
        // Should average to approximately the same value
        assert!(result.pixel(25, 25)[0] > 100 && result.pixel(25, 25)[0] < 150);
    }

    #[test]
    fn test_resize_pixel_averaging_upscale_falls_back() {
        let img = ImageBuffer::blank(20, 20, ColorSpace::Rgb8);
        let filter = ResizeFilter::new(40, 40, ResizeAlgorithm::PixelAveraging);
        let result = filter.apply(&img).unwrap();
        // Should fall back to bilinear for upscaling
        assert_eq!(result.width, 40);
        assert_eq!(result.height, 40);
    }

    #[test]
    fn test_resize_halftone() {
        let mut img = ImageBuffer::blank(20, 20, ColorSpace::Rgb8);
        // Create a gradient
        for y in 0..20 {
            for x in 0..20 {
                let val = ((x + y) * 6) as u8;
                img.set_pixel(x, y, &[val, val, val]);
            }
        }
        let filter = ResizeFilter::new(10, 10, ResizeAlgorithm::Halftone);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.width, 10);
        assert_eq!(result.height, 10);
        // Halftone should produce only 0 or 255 values
        assert!(result.data.iter().all(|&v| v == 0 || v == 255));
    }

    #[test]
    fn test_resize_algorithm_equality() {
        // Test PartialEq/Eq derives
        assert_eq!(ResizeAlgorithm::Nearest, ResizeAlgorithm::Nearest);
        assert_ne!(ResizeAlgorithm::Nearest, ResizeAlgorithm::Bilinear);
        assert_eq!(ResizeAlgorithm::Lanczos3, ResizeAlgorithm::Lanczos3);
        assert_ne!(ResizeAlgorithm::Bicubic, ResizeAlgorithm::PixelAveraging);
    }

    #[test]
    fn test_resize_lanczos3_grayscale() {
        let img = ImageBuffer::blank(50, 50, ColorSpace::Grayscale);
        let filter = ResizeFilter::new(25, 25, ResizeAlgorithm::Lanczos3);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.color_space, ColorSpace::Grayscale);
        assert_eq!(result.size_bytes(), 25 * 25);
    }

    #[test]
    fn test_resize_bicubic_rgba() {
        let img = ImageBuffer::blank(40, 40, ColorSpace::Rgba8);
        let filter = ResizeFilter::new(20, 20, ResizeAlgorithm::Bicubic);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.color_space, ColorSpace::Rgba8);
        assert_eq!(result.size_bytes(), 20 * 20 * 4);
    }
}
