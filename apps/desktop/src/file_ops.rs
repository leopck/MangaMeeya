use mm_image::{ColorSpace, ImageBuffer};
use std::path::Path;

/// Save current page image to a file with format auto-detected from extension.
pub fn save_image(img: &ImageBuffer, path: &Path) -> Result<(), String> {
    let (w, h) = (img.width, img.height);

    // Convert to image::RgbImage or RgbaImage
    let dyn_img = match img.color_space {
        ColorSpace::Rgb8 => {
            let rgb = image::RgbImage::from_raw(w, h, img.data.clone())
                .ok_or("Failed to create RGB image")?;
            image::DynamicImage::ImageRgb8(rgb)
        }
        ColorSpace::Rgba8 => {
            let rgba = image::RgbaImage::from_raw(w, h, img.data.clone())
                .ok_or("Failed to create RGBA image")?;
            image::DynamicImage::ImageRgba8(rgba)
        }
        ColorSpace::Grayscale => {
            let gray = image::GrayImage::from_raw(w, h, img.data.clone())
                .ok_or("Failed to create grayscale image")?;
            image::DynamicImage::ImageLuma8(gray)
        }
    };

    dyn_img.save(path).map_err(|e| format!("Save error: {e}"))
}

/// Copy current page image to system clipboard.
pub fn copy_to_clipboard(img: &ImageBuffer) -> Result<(), String> {
    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("Clipboard error: {e}"))?;

    // Convert to RGBA for clipboard
    let (w, h) = (img.width as usize, img.height as usize);
    let rgba_data = match img.color_space {
        ColorSpace::Rgba8 => img.data.clone(),
        ColorSpace::Rgb8 => {
            let mut rgba = Vec::with_capacity(w * h * 4);
            for i in 0..(w * h) {
                let o = i * 3;
                rgba.push(img.data[o]);
                rgba.push(img.data[o + 1]);
                rgba.push(img.data[o + 2]);
                rgba.push(255);
            }
            rgba
        }
        ColorSpace::Grayscale => {
            let mut rgba = Vec::with_capacity(w * h * 4);
            for &v in &img.data[..w * h] {
                rgba.push(v);
                rgba.push(v);
                rgba.push(v);
                rgba.push(255);
            }
            rgba
        }
    };

    let img_data = arboard::ImageData {
        width: w,
        height: h,
        bytes: std::borrow::Cow::Owned(rgba_data),
    };

    clipboard.set_image(img_data).map_err(|e| format!("Clipboard set error: {e}"))
}

/// Quick save configuration for 4 preset slots.
#[derive(Debug, Clone, Default)]
pub struct QuickSaveConfig {
    pub slots: [Option<QuickSaveSlot>; 4],
}

#[derive(Debug, Clone)]
pub struct QuickSaveSlot {
    pub folder: std::path::PathBuf,
    pub format: String, // "jpg", "png", "webp", etc.
    #[allow(dead_code)]
    pub quality: u8,
}
