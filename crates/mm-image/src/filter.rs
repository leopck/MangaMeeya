pub mod crop;
pub mod flip;
pub mod grayscale;
pub mod resize;
pub mod rotate;
pub mod sharpen;

use crate::{ImageBuffer, ImageResult};

/// Trait for image processing filters.
pub trait ImageFilter: Send + Sync {
    fn name(&self) -> &str;
    fn apply(&self, image: &ImageBuffer) -> ImageResult<ImageBuffer>;
}

/// A pipeline of filters applied sequentially.
pub struct ImagePipeline {
    filters: Vec<Box<dyn ImageFilter>>,
}

impl ImagePipeline {
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    pub fn add(&mut self, filter: Box<dyn ImageFilter>) {
        self.filters.push(filter);
    }

    pub fn apply(&self, image: &ImageBuffer) -> ImageResult<ImageBuffer> {
        let mut current = image.clone();
        for filter in &self.filters {
            current = filter.apply(&current)?;
        }
        Ok(current)
    }

    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }
}

impl Default for ImagePipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ColorSpace, ImageBuffer};

    #[test]
    fn test_pipeline_empty() {
        let pipeline = ImagePipeline::new();
        assert!(pipeline.is_empty());
        let img = ImageBuffer::blank(10, 10, ColorSpace::Rgb8);
        let result = pipeline.apply(&img).unwrap();
        assert_eq!(result.data, img.data);
    }

    #[test]
    fn test_pipeline_chain() {
        let mut pipeline = ImagePipeline::new();
        pipeline.add(Box::new(crop::CropFilter::new(0, 0, 5, 5)));
        pipeline.add(Box::new(grayscale::GrayscaleFilter));
        let img = ImageBuffer::blank(10, 10, ColorSpace::Rgb8);
        let result = pipeline.apply(&img).unwrap();
        assert_eq!(result.width, 5);
        assert_eq!(result.height, 5);
        assert_eq!(result.color_space, ColorSpace::Grayscale);
    }
}
