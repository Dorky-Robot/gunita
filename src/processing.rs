use std::path::Path;

use image::{ImageBuffer, RgbImage};
use imagepipe::Pipeline;
use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Minimal edit params for RAW processing (exposure + white balance only).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EditParams {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exposure: Option<f64>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub white_balance: Option<f64>,
}

/// Apply EditParams to an imagepipe Pipeline before processing.
fn apply_edits(pipeline: &mut Pipeline, edits: &EditParams) {
    if let Some(ev) = edits.exposure {
        pipeline.ops.basecurve.exposure = ev as f32;
    }

    if let Some(temp) = edits.white_balance {
        pipeline.ops.tolab.set_temp(temp as f32, 1.0);
    }
}

/// Process a RAW file and return an 8-bit sRGB image buffer.
pub fn process_raw(
    raw_path: &Path,
    edits: &EditParams,
    max_width: usize,
    max_height: usize,
) -> Result<RgbImage, AppError> {
    let path_str = raw_path
        .to_str()
        .ok_or_else(|| AppError::Processing("Invalid file path".into()))?;

    let mut pipeline = Pipeline::new_from_file(path_str)
        .map_err(|e| AppError::Processing(format!("Failed to open RAW: {e}")))?;

    apply_edits(&mut pipeline, edits);

    let srgb = pipeline
        .output_8bit(None)
        .map_err(|e| AppError::Processing(format!("Pipeline error: {e}")))?;

    let img: RgbImage =
        ImageBuffer::from_raw(srgb.width as u32, srgb.height as u32, srgb.data)
            .ok_or_else(|| AppError::Processing("Failed to create image buffer".into()))?;

    let img = resize_if_needed(img, max_width, max_height);
    Ok(img)
}

/// Encode an RgbImage as JPEG bytes.
pub fn encode_jpeg(img: &RgbImage, quality: u8) -> Result<Vec<u8>, AppError> {
    let mut buf = std::io::Cursor::new(Vec::new());
    let encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut buf, quality);
    image::ImageEncoder::write_image(
        encoder,
        img.as_raw(),
        img.width(),
        img.height(),
        image::ExtendedColorType::Rgb8,
    )?;
    Ok(buf.into_inner())
}

fn resize_if_needed(img: RgbImage, max_width: usize, max_height: usize) -> RgbImage {
    if max_width == 0 && max_height == 0 {
        return img;
    }

    let (w, h) = (img.width() as usize, img.height() as usize);
    let target_w = if max_width > 0 { max_width } else { w };
    let target_h = if max_height > 0 { max_height } else { h };

    if w <= target_w && h <= target_h {
        return img;
    }

    let scale = f64::min(target_w as f64 / w as f64, target_h as f64 / h as f64);
    let new_w = (w as f64 * scale) as u32;
    let new_h = (h as f64 * scale) as u32;

    let dynamic = image::DynamicImage::ImageRgb8(img);
    dynamic
        .resize(new_w, new_h, image::imageops::FilterType::Lanczos3)
        .to_rgb8()
}
