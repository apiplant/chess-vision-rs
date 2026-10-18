//! Text-line recognition with PaddleOCR's PP-OCRv5 mobile recognizer (Apache-2.0) on ONNX Runtime.
//!
//! Only the recognition model is used (no text detector): callers pass crops that already contain
//! one short piece of text. Crops are batched into a single `session.run`.

use anyhow::{anyhow, Context, Result};
use image::{imageops::FilterType, DynamicImage, GrayImage, Luma, RgbImage};
use ndarray::Array4;
use ort::session::Session;
use ort::value::TensorRef;
use std::path::Path;

const HEIGHT: u32 = 48;

pub struct Recognizer {
    session: Session,
    /// CTC classes: index 0 is the blank, then the dictionary, then a trailing space.
    chars: Vec<String>,
}

impl Recognizer {
    pub fn load(model: impl AsRef<Path>, dict: impl AsRef<Path>) -> Result<Self> {
        let session = crate::runtime::session_builder()?
            .commit_from_file(model.as_ref())
            .map_err(|e| anyhow!("loading {}: {e}", model.as_ref().display()))?;
        let dict = std::fs::read_to_string(dict.as_ref())
            .with_context(|| format!("reading {}", dict.as_ref().display()))?;
        let mut chars = vec![String::new()];
        chars.extend(dict.lines().map(str::to_owned));
        chars.push(" ".into());
        Ok(Self { session, chars })
    }

    /// Recognise one string per crop.
    pub fn recognize(&mut self, crops: &[DynamicImage]) -> Result<Vec<String>> {
        if crops.is_empty() {
            return Ok(vec![]);
        }
        let prepared: Vec<RgbImage> = crops.iter().map(prepare).collect();
        let width = prepared.iter().map(|p| p.width()).max().unwrap() as usize;
        let mut input = Array4::<f32>::ones((prepared.len(), 3, HEIGHT as usize, width)); // 1.0 = white padding
        for (b, img) in prepared.iter().enumerate() {
            for (x, y, px) in img.enumerate_pixels() {
                for c in 0..3 {
                    input[[b, c, y as usize, x as usize]] = (px.0[c] as f32 / 255.0 - 0.5) / 0.5;
                }
            }
        }
        let tensor = TensorRef::from_array_view(input.view()).map_err(|e| anyhow!("{e}"))?;
        let outputs = self
            .session
            .run(ort::inputs!["x" => tensor])
            .map_err(|e| anyhow!("ocr inference: {e}"))?;
        let (shape, data) = outputs[0].try_extract_tensor::<f32>().map_err(|e| anyhow!("{e}"))?;
        let (steps, classes) = (shape[1] as usize, shape[2] as usize);
        Ok((0..prepared.len())
            .map(|b| {
                let mut text = String::new();
                let mut prev = 0usize;
                for t in 0..steps {
                    let row = &data[(b * steps + t) * classes..(b * steps + t + 1) * classes];
                    let best = row.iter().enumerate().max_by(|a, c| a.1.total_cmp(c.1)).map_or(0, |(i, _)| i);
                    if best != prev && best != 0 {
                        if let Some(c) = self.chars.get(best) {
                            text.push_str(c);
                        }
                    }
                    prev = best;
                }
                text
            })
            .collect())
    }
}

/// Grayscale, stretch contrast (1% clip), white margin, resize to the model's 48 px height.
fn prepare(img: &DynamicImage) -> RgbImage {
    let mut g = img.to_luma8();
    let mut hist = [0u32; 256];
    g.pixels().for_each(|p| hist[p.0[0] as usize] += 1);
    let clip = (g.width() * g.height() / 100).max(1);
    let (mut lo, mut acc) = (0usize, 0u32);
    while lo < 255 && acc + hist[lo] <= clip {
        acc += hist[lo];
        lo += 1;
    }
    let (mut hi, mut acc) = (255usize, 0u32);
    while hi > lo && acc + hist[hi] <= clip {
        acc += hist[hi];
        hi -= 1;
    }
    if hi > lo {
        for p in g.pixels_mut() {
            p.0[0] = (((p.0[0] as f32 - lo as f32) * 255.0 / (hi - lo) as f32).clamp(0.0, 255.0)) as u8;
        }
    }
    let mut padded = GrayImage::from_pixel(g.width() + 12, g.height() + 12, Luma([255]));
    image::imageops::replace(&mut padded, &g, 6, 6);
    let w = ((padded.width() as f32 * HEIGHT as f32 / padded.height() as f32).round() as u32).max(24);
    DynamicImage::ImageLuma8(image::imageops::resize(&padded, w, HEIGHT, FilterType::CatmullRom)).to_rgb8()
}
