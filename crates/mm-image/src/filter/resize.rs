use crate::{ImageBuffer, ImageError, ImageResult};
use super::ImageFilter;

#[derive(Debug, Clone, Copy)]
pub enum ResizeAlgorithm {
    Nearest,
    Bilinear,
    // Lanczos3 and Bicubic will be added with SIMD optimization
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
    pub fn fit(max_width: u32, max_height: u32, src_width: u32, src_height: u32, algorithm: ResizeAlgorithm) -> Self {
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
}
