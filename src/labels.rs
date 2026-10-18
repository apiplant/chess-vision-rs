//! Board orientation from printed coordinate labels (ranks 1-8, files a-h).
//!
//! Only the *direction* of the sequences matters, so a few misread or missing labels are fine:
//! we vote over all pairs of readable labels. Assumes the common screenshot layout with rank digits
//! in the top-left corner of the left-column squares and file letters in the bottom-right corner of
//! the bottom-row squares. Returns `None` if the labels aren't readable.

use anyhow::Result;
use image::DynamicImage;

use crate::board::Orientation;
use crate::ocr::Recognizer;

/// Board bounds in image pixels: (x1, y1, x2, y2).
pub type BoardBox = (f32, f32, f32, f32);

/// Votes (normal, flipped) from pairs of readable labels. `normal_sign` is the expected sign of
/// `value[j] - value[i]` for `i < j` when white is at the bottom.
fn vote(labels: &[Option<u8>], normal_sign: i32) -> (u32, u32) {
    let (mut normal, mut flipped) = (0, 0);
    for i in 0..labels.len() {
        for j in i + 1..labels.len() {
            if let (Some(a), Some(b)) = (labels[i], labels[j]) {
                match (b as i32 - a as i32).signum() * normal_sign {
                    1 => normal += 1,
                    -1 => flipped += 1,
                    _ => {}
                }
            }
        }
    }
    (normal, flipped)
}

/// OCR output is noisy ("|3", "I5", "C"): take the last character that is a valid label.
fn parse_rank(s: &str) -> Option<u8> {
    s.chars().rev().find(|c| ('1'..='8').contains(c)).map(|c| c as u8 - b'0')
}

fn parse_file(s: &str) -> Option<u8> {
    s.chars().rev().map(|c| c.to_ascii_lowercase()).find(|c| ('a'..='h').contains(c)).map(|c| c as u8 - b'a' + 1)
}

pub fn orientation_from_labels(img: &DynamicImage, board: BoardBox, ocr: &mut Recognizer) -> Result<Option<Orientation>> {
    let (x1, y1, x2, y2) = board;
    let (sw, sh) = ((x2 - x1) / 8.0, (y2 - y1) / 8.0);
    let crop = |x: f32, y: f32| {
        let (x, y) = (x.clamp(0.0, img.width() as f32 - 2.0), y.clamp(0.0, img.height() as f32 - 2.0));
        img.crop_imm(x as u32, y as u32, (sw * 0.3).max(2.0) as u32, (sh * 0.3).max(2.0) as u32)
    };
    // Eight rank crops (left column, top to bottom), then eight file crops (bottom row, left to right).
    let mut crops: Vec<DynamicImage> = (0..8).map(|r| crop(x1, y1 + r as f32 * sh)).collect();
    crops.extend((0..8).map(|c| crop(x1 + (c + 1) as f32 * sw - sw * 0.3, y2 - sh * 0.3)));
    let text = ocr.recognize(&crops)?;
    let ranks: Vec<_> = text[..8].iter().map(|s| parse_rank(s)).collect();
    let files: Vec<_> = text[8..].iter().map(|s| parse_file(s)).collect();
    if std::env::var_os("CHESS_VISION_DEBUG").is_some() {
        eprintln!("rank labels: {:?} -> {ranks:?}\nfile labels: {:?} -> {files:?}", &text[..8], &text[8..]);
    }

    // White at bottom: digits go 8..1 top to bottom (descending), letters a..h left to right (ascending).
    let (rn, rf) = vote(&ranks, -1);
    let (fn_, ff) = vote(&files, 1);
    let (normal, flipped) = (rn + fn_, rf + ff);
    let total = normal + flipped;
    Ok(if total < 3 {
        None
    } else if normal * 4 >= total * 3 {
        Some(Orientation::WhiteBottom)
    } else if flipped * 4 >= total * 3 {
        Some(Orientation::BlackBottom)
    } else {
        None
    })
}
