use super::ImageFilter;
use crate::{ImageBuffer, ImageResult};

pub struct FlipHorizontal;
pub struct FlipVertical;

impl ImageFilter for FlipHorizontal {
    fn name(&self) -> &str {
        "FlipHorizontal"
    }

    fn apply(&self, image: &ImageBuffer) -> ImageResult<ImageBuffer> {
        let bpp = image.color_space.bytes_per_pixel();
        let stride = image.stride();
        let mut data = vec![0u8; image.data.len()];

        for y in 0..image.height as usize {
            for x in 0..image.width as usize {
                let src = y * stride + x * bpp;
                let dst = y * stride + (image.width as usize - 1 - x) * bpp;
                data[dst..dst + bpp].copy_from_slice(&image.data[src..src + bpp]);
            }
        }

        Ok(ImageBuffer::new(
            data,
            image.width,
            image.height,
            image.color_space,
        ))
    }
}

impl ImageFilter for FlipVertical {
    fn name(&self) -> &str {
        "FlipVertical"
    }

    fn apply(&self, image: &ImageBuffer) -> ImageResult<ImageBuffer> {
        let stride = image.stride();
        let mut data = vec![0u8; image.data.len()];

        for y in 0..image.height as usize {
            let src_row = y * stride;
            let dst_row = (image.height as usize - 1 - y) * stride;
            data[dst_row..dst_row + stride].copy_from_slice(&image.data[src_row..src_row + stride]);
        }

        Ok(ImageBuffer::new(
            data,
            image.width,
            image.height,
            image.color_space,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ColorSpace;

    #[test]
    fn test_flip_horizontal() {
        let mut img = ImageBuffer::blank(3, 1, ColorSpace::Rgb8);
        img.set_pixel(0, 0, &[255, 0, 0]);
        img.set_pixel(2, 0, &[0, 0, 255]);
        let result = FlipHorizontal.apply(&img).unwrap();
        assert_eq!(result.pixel(0, 0), &[0, 0, 255]);
        assert_eq!(result.pixel(2, 0), &[255, 0, 0]);
    }

    #[test]
    fn test_flip_vertical() {
        let mut img = ImageBuffer::blank(1, 3, ColorSpace::Rgb8);
        img.set_pixel(0, 0, &[255, 0, 0]);
        img.set_pixel(0, 2, &[0, 0, 255]);
        let result = FlipVertical.apply(&img).unwrap();
        assert_eq!(result.pixel(0, 0), &[0, 0, 255]);
        assert_eq!(result.pixel(0, 2), &[255, 0, 0]);
    }

    #[test]
    fn test_flip_h_twice_identity() {
        let mut img = ImageBuffer::blank(5, 5, ColorSpace::Rgb8);
        img.set_pixel(0, 0, &[1, 2, 3]);
        img.set_pixel(4, 4, &[4, 5, 6]);
        let once = FlipHorizontal.apply(&img).unwrap();
        let twice = FlipHorizontal.apply(&once).unwrap();
        assert_eq!(twice.data, img.data);
    }

    #[test]
    fn test_flip_v_twice_identity() {
        let mut img = ImageBuffer::blank(5, 5, ColorSpace::Rgb8);
        img.set_pixel(0, 0, &[10, 20, 30]);
        img.set_pixel(4, 4, &[40, 50, 60]);
        let once = FlipVertical.apply(&img).unwrap();
        let twice = FlipVertical.apply(&once).unwrap();
        assert_eq!(twice.data, img.data);
    }

    #[test]
    fn test_flip_1x1() {
        let img = ImageBuffer::new(vec![42, 84, 126], 1, 1, ColorSpace::Rgb8);
        let h = FlipHorizontal.apply(&img).unwrap();
        let v = FlipVertical.apply(&img).unwrap();
        assert_eq!(h.data, img.data);
        assert_eq!(v.data, img.data);
    }
}
