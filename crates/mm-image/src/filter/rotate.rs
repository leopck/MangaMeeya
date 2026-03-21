use super::ImageFilter;
use crate::{ImageBuffer, ImageResult};

#[derive(Debug, Clone, Copy)]
pub enum Rotation {
    Cw90,
    Cw180,
    Cw270,
}

pub struct RotateFilter {
    rotation: Rotation,
}

impl RotateFilter {
    pub fn new(rotation: Rotation) -> Self {
        Self { rotation }
    }
}

impl ImageFilter for RotateFilter {
    fn name(&self) -> &str {
        "Rotate"
    }

    fn apply(&self, image: &ImageBuffer) -> ImageResult<ImageBuffer> {
        let bpp = image.color_space.bytes_per_pixel();
        let (w, h) = (image.width as usize, image.height as usize);

        let (new_w, new_h) = match self.rotation {
            Rotation::Cw90 | Rotation::Cw270 => (h, w),
            Rotation::Cw180 => (w, h),
        };

        let src_stride = w * bpp;
        let dst_stride = new_w * bpp;
        let mut data = vec![0u8; new_w * new_h * bpp];

        for y in 0..h {
            for x in 0..w {
                let (dx, dy) = match self.rotation {
                    Rotation::Cw90 => (h - 1 - y, x),
                    Rotation::Cw180 => (w - 1 - x, h - 1 - y),
                    Rotation::Cw270 => (y, w - 1 - x),
                };

                let src = y * src_stride + x * bpp;
                let dst = dy * dst_stride + dx * bpp;
                data[dst..dst + bpp].copy_from_slice(&image.data[src..src + bpp]);
            }
        }

        Ok(ImageBuffer::new(
            data,
            new_w as u32,
            new_h as u32,
            image.color_space,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ColorSpace;

    #[test]
    fn test_rotate_90_dimensions() {
        let img = ImageBuffer::blank(100, 50, ColorSpace::Rgb8);
        let result = RotateFilter::new(Rotation::Cw90).apply(&img).unwrap();
        assert_eq!(result.width, 50);
        assert_eq!(result.height, 100);
    }

    #[test]
    fn test_rotate_180_dimensions() {
        let img = ImageBuffer::blank(100, 50, ColorSpace::Rgb8);
        let result = RotateFilter::new(Rotation::Cw180).apply(&img).unwrap();
        assert_eq!(result.width, 100);
        assert_eq!(result.height, 50);
    }

    #[test]
    fn test_rotate_270_dimensions() {
        let img = ImageBuffer::blank(100, 50, ColorSpace::Rgb8);
        let result = RotateFilter::new(Rotation::Cw270).apply(&img).unwrap();
        assert_eq!(result.width, 50);
        assert_eq!(result.height, 100);
    }

    #[test]
    fn test_rotate_180_pixel() {
        let mut img = ImageBuffer::blank(3, 3, ColorSpace::Rgb8);
        img.set_pixel(0, 0, &[255, 0, 0]);
        let result = RotateFilter::new(Rotation::Cw180).apply(&img).unwrap();
        assert_eq!(result.pixel(2, 2), &[255, 0, 0]);
    }

    #[test]
    fn test_rotate_360_identity() {
        let mut img = ImageBuffer::blank(5, 7, ColorSpace::Rgb8);
        img.set_pixel(1, 2, &[10, 20, 30]);
        img.set_pixel(3, 4, &[40, 50, 60]);

        let r90 = RotateFilter::new(Rotation::Cw90);
        let step1 = r90.apply(&img).unwrap();
        let step2 = r90.apply(&step1).unwrap();
        let step3 = r90.apply(&step2).unwrap();
        let step4 = r90.apply(&step3).unwrap();

        assert_eq!(step4.width, img.width);
        assert_eq!(step4.height, img.height);
        assert_eq!(step4.data, img.data);
    }

    #[test]
    fn test_rotate_90_pixel() {
        let mut img = ImageBuffer::blank(3, 2, ColorSpace::Rgb8);
        // Top-right corner
        img.set_pixel(2, 0, &[255, 0, 0]);
        let result = RotateFilter::new(Rotation::Cw90).apply(&img).unwrap();
        // After 90 CW, top-right becomes bottom-right
        assert_eq!(result.pixel(1, 2), &[255, 0, 0]);
    }

    #[test]
    fn test_rotate_1x1() {
        let img = ImageBuffer::new(vec![42, 84, 126], 1, 1, ColorSpace::Rgb8);
        let result = RotateFilter::new(Rotation::Cw90).apply(&img).unwrap();
        assert_eq!(result.pixel(0, 0), &[42, 84, 126]);
    }
}
