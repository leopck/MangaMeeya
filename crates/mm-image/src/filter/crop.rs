use crate::{ColorSpace, ImageBuffer, ImageError, ImageResult};
use super::ImageFilter;

pub struct CropFilter {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl CropFilter {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }
}

impl ImageFilter for CropFilter {
    fn name(&self) -> &str {
        "Crop"
    }

    fn apply(&self, image: &ImageBuffer) -> ImageResult<ImageBuffer> {
        if self.x + self.width > image.width || self.y + self.height > image.height {
            return Err(ImageError::Filter(format!(
                "Crop region ({},{} {}x{}) exceeds image ({}x{})",
                self.x, self.y, self.width, self.height, image.width, image.height
            )));
        }

        if self.width == 0 || self.height == 0 {
            return Err(ImageError::InvalidDimensions {
                width: self.width,
                height: self.height,
            });
        }

        let bpp = image.color_space.bytes_per_pixel();
        let src_stride = image.stride();
        let dst_stride = self.width as usize * bpp;
        let mut data = vec![0u8; dst_stride * self.height as usize];

        for row in 0..self.height as usize {
            let src_offset = (self.y as usize + row) * src_stride + self.x as usize * bpp;
            let dst_offset = row * dst_stride;
            data[dst_offset..dst_offset + dst_stride]
                .copy_from_slice(&image.data[src_offset..src_offset + dst_stride]);
        }

        Ok(ImageBuffer::new(data, self.width, self.height, image.color_space))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crop_basic() {
        let img = ImageBuffer::blank(100, 100, ColorSpace::Rgb8);
        let filter = CropFilter::new(10, 10, 50, 50);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.width, 50);
        assert_eq!(result.height, 50);
    }

    #[test]
    fn test_crop_full() {
        let img = ImageBuffer::blank(100, 100, ColorSpace::Rgb8);
        let filter = CropFilter::new(0, 0, 100, 100);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.data, img.data);
    }

    #[test]
    fn test_crop_out_of_bounds() {
        let img = ImageBuffer::blank(100, 100, ColorSpace::Rgb8);
        let filter = CropFilter::new(50, 50, 60, 60);
        assert!(filter.apply(&img).is_err());
    }

    #[test]
    fn test_crop_pixel_correctness() {
        let mut img = ImageBuffer::blank(10, 10, ColorSpace::Rgb8);
        img.set_pixel(5, 5, &[255, 0, 0]);
        let filter = CropFilter::new(5, 5, 3, 3);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.pixel(0, 0), &[255, 0, 0]);
    }

    #[test]
    fn test_crop_rgba() {
        let img = ImageBuffer::blank(50, 50, ColorSpace::Rgba8);
        let filter = CropFilter::new(10, 10, 20, 20);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.width, 20);
        assert_eq!(result.color_space, ColorSpace::Rgba8);
    }

    #[test]
    fn test_crop_1x1() {
        let mut img = ImageBuffer::blank(10, 10, ColorSpace::Rgb8);
        img.set_pixel(3, 7, &[42, 84, 126]);
        let filter = CropFilter::new(3, 7, 1, 1);
        let result = filter.apply(&img).unwrap();
        assert_eq!(result.pixel(0, 0), &[42, 84, 126]);
    }
}
