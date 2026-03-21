/// Color space of the image data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorSpace {
    Rgb8,
    Rgba8,
    Grayscale,
}

impl ColorSpace {
    /// Bytes per pixel for this color space.
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            ColorSpace::Rgb8 => 3,
            ColorSpace::Rgba8 => 4,
            ColorSpace::Grayscale => 1,
        }
    }
}

/// A decoded image held in CPU memory.
#[derive(Debug, Clone)]
pub struct ImageBuffer {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub color_space: ColorSpace,
}

impl ImageBuffer {
    /// Create a new image buffer, validating dimensions.
    pub fn new(data: Vec<u8>, width: u32, height: u32, color_space: ColorSpace) -> Self {
        debug_assert_eq!(
            data.len(),
            width as usize * height as usize * color_space.bytes_per_pixel()
        );
        Self {
            data,
            width,
            height,
            color_space,
        }
    }

    /// Stride (bytes per row).
    pub fn stride(&self) -> usize {
        self.width as usize * self.color_space.bytes_per_pixel()
    }

    /// Total size in bytes.
    pub fn size_bytes(&self) -> usize {
        self.data.len()
    }

    /// Get pixel at (x, y) as a slice of bytes.
    pub fn pixel(&self, x: u32, y: u32) -> &[u8] {
        let bpp = self.color_space.bytes_per_pixel();
        let offset = (y as usize * self.stride()) + (x as usize * bpp);
        &self.data[offset..offset + bpp]
    }

    /// Set pixel at (x, y).
    pub fn set_pixel(&mut self, x: u32, y: u32, pixel: &[u8]) {
        let bpp = self.color_space.bytes_per_pixel();
        let offset = (y as usize * self.stride()) + (x as usize * bpp);
        self.data[offset..offset + bpp].copy_from_slice(pixel);
    }

    /// Create a blank (zero-filled) image.
    pub fn blank(width: u32, height: u32, color_space: ColorSpace) -> Self {
        let size = width as usize * height as usize * color_space.bytes_per_pixel();
        Self {
            data: vec![0u8; size],
            width,
            height,
            color_space,
        }
    }

    /// Check if this image is portrait (height > width).
    pub fn is_portrait(&self) -> bool {
        self.height > self.width
    }

    /// Check if this image is landscape (width > height).
    pub fn is_landscape(&self) -> bool {
        self.width > self.height
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_rgb() {
        let data = vec![0u8; 100 * 100 * 3];
        let img = ImageBuffer::new(data, 100, 100, ColorSpace::Rgb8);
        assert_eq!(img.width, 100);
        assert_eq!(img.height, 100);
        assert_eq!(img.stride(), 300);
        assert_eq!(img.size_bytes(), 30000);
    }

    #[test]
    fn test_new_rgba() {
        let data = vec![255u8; 10 * 10 * 4];
        let img = ImageBuffer::new(data, 10, 10, ColorSpace::Rgba8);
        assert_eq!(img.stride(), 40);
        assert_eq!(img.size_bytes(), 400);
    }

    #[test]
    fn test_pixel_access() {
        let mut data = vec![0u8; 3 * 3 * 3];
        // Set pixel (1, 1) to red
        data[3 * 3 + 3] = 255; // R
        let img = ImageBuffer::new(data, 3, 3, ColorSpace::Rgb8);
        assert_eq!(img.pixel(1, 1), &[255, 0, 0]);
        assert_eq!(img.pixel(0, 0), &[0, 0, 0]);
    }

    #[test]
    fn test_set_pixel() {
        let mut img = ImageBuffer::blank(3, 3, ColorSpace::Rgb8);
        img.set_pixel(2, 2, &[255, 128, 0]);
        assert_eq!(img.pixel(2, 2), &[255, 128, 0]);
    }

    #[test]
    fn test_blank() {
        let img = ImageBuffer::blank(50, 50, ColorSpace::Rgba8);
        assert_eq!(img.size_bytes(), 50 * 50 * 4);
        assert!(img.data.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_portrait_landscape() {
        let portrait = ImageBuffer::blank(100, 200, ColorSpace::Rgb8);
        assert!(portrait.is_portrait());
        assert!(!portrait.is_landscape());

        let landscape = ImageBuffer::blank(200, 100, ColorSpace::Rgb8);
        assert!(!landscape.is_portrait());
        assert!(landscape.is_landscape());

        let square = ImageBuffer::blank(100, 100, ColorSpace::Rgb8);
        assert!(!square.is_portrait());
        assert!(!square.is_landscape());
    }

    #[test]
    fn test_grayscale() {
        let img = ImageBuffer::blank(10, 10, ColorSpace::Grayscale);
        assert_eq!(img.color_space.bytes_per_pixel(), 1);
        assert_eq!(img.size_bytes(), 100);
        assert_eq!(img.pixel(0, 0), &[0]);
    }

    #[test]
    fn test_1x1() {
        let img = ImageBuffer::new(vec![255, 0, 128], 1, 1, ColorSpace::Rgb8);
        assert_eq!(img.pixel(0, 0), &[255, 0, 128]);
    }
}
