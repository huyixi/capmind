#[cfg(target_os = "macos")]
use arboard::Clipboard;
#[cfg(target_os = "macos")]
use image::ColorType;
#[cfg(target_os = "macos")]
use image::ImageEncoder;
#[cfg(target_os = "macos")]
use image::codecs::png::PngEncoder;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

pub const MAX_PASTED_IMAGES: usize = 9;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct PastedImage {
    pub filename: String,
    pub png_bytes: Vec<u8>,
}

impl PastedImage {
    pub fn byte_len(&self) -> usize {
        self.png_bytes.len()
    }
}

#[cfg(target_os = "macos")]
pub fn read_image_from_clipboard(next_index: usize) -> Result<PastedImage, AppError> {
    let mut clipboard = Clipboard::new().map_err(|err| {
        AppError::InvalidInput(format!("Failed to access macOS clipboard: {err}"))
    })?;
    let image = clipboard.get_image().map_err(|err| match err {
        arboard::Error::ContentNotAvailable => {
            AppError::InvalidInput("Clipboard does not contain an image.".to_string())
        }
        other => AppError::InvalidInput(format!("Failed to read clipboard image: {other}")),
    })?;

    let mut png_bytes = Vec::new();
    let encoder = PngEncoder::new(&mut png_bytes);
    encoder
        .write_image(
            image.bytes.as_ref(),
            image.width as u32,
            image.height as u32,
            ColorType::Rgba8.into(),
        )
        .map_err(|err| AppError::Api(format!("Failed to encode clipboard image as PNG: {err}")))?;

    Ok(PastedImage {
        filename: format!("pasted-image-{next_index}.png"),
        png_bytes,
    })
}

#[cfg(not(target_os = "macos"))]
pub fn read_image_from_clipboard(_next_index: usize) -> Result<PastedImage, AppError> {
    Err(AppError::InvalidInput(
        "Image paste is currently supported on macOS only.".to_string(),
    ))
}
