//! Detection types, letterbox preprocessing and NMS.

use image::{imageops::FilterType, DynamicImage, Rgb, RgbImage};
use serde::Serialize;

pub const INPUT_SIZE: usize = 640;
pub const NUM_CLASSES: usize = 13;
pub const CLASS_NAMES: [&str; NUM_CLASSES] = [
    "board", "white_king", "white_queen", "white_rook", "white_bishop", "white_knight",
    "white_pawn", "black_king", "black_queen", "black_rook", "black_bishop", "black_knight",
    "black_pawn",
];

/// A detection in original-image pixel coordinates.
#[derive(Debug, Clone, Serialize)]
pub struct Detection {
    pub class: usize,
    pub label: &'static str,
    pub confidence: f32,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl Detection {
    pub fn w(&self) -> f32 {
        self.x2 - self.x1
    }
    pub fn h(&self) -> f32 {
        self.y2 - self.y1
    }
    fn iou(&self, o: &Detection) -> f32 {
        let iw = (self.x2.min(o.x2) - self.x1.max(o.x1)).max(0.0);
        let ih = (self.y2.min(o.y2) - self.y1.max(o.y1)).max(0.0);
        let inter = iw * ih;
        inter / (self.w() * self.h() + o.w() * o.h() - inter + 1e-9)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Letterbox {
    scale: f32,
    pad_x: f32,
    pad_y: f32,
}

impl Letterbox {
    /// Map a box from 640x640 network space back to original-image pixels.
    pub fn unmap(&self, x1: f32, y1: f32, x2: f32, y2: f32) -> (f32, f32, f32, f32) {
        let (ux, uy) = (|x: f32| (x - self.pad_x) / self.scale, |y: f32| (y - self.pad_y) / self.scale);
        (ux(x1), uy(y1), ux(x2), uy(y2))
    }
}

/// Resize keeping aspect ratio and pad with gray (114) to 640x640.
pub fn letterbox(img: &DynamicImage) -> (RgbImage, Letterbox) {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let scale = (INPUT_SIZE as f32 / w).min(INPUT_SIZE as f32 / h);
    let (nw, nh) = ((w * scale).round() as u32, (h * scale).round() as u32);
    let (px, py) = ((INPUT_SIZE as u32 - nw) / 2, (INPUT_SIZE as u32 - nh) / 2);
    let resized = img.resize_exact(nw, nh, FilterType::Triangle).to_rgb8();
    let mut canvas = RgbImage::from_pixel(INPUT_SIZE as u32, INPUT_SIZE as u32, Rgb([114; 3]));
    image::imageops::replace(&mut canvas, &resized, px as i64, py as i64);
    (canvas, Letterbox { scale, pad_x: px as f32, pad_y: py as f32 })
}

/// Class-aware non-maximum suppression.
pub fn nms(mut dets: Vec<Detection>, iou_thr: f32) -> Vec<Detection> {
    dets.sort_by(|a, b| b.confidence.total_cmp(&a.confidence));
    let mut keep: Vec<Detection> = Vec::new();
    for d in dets {
        if keep.iter().all(|k| k.class != d.class || k.iou(&d) < iou_thr) {
            keep.push(d);
        }
    }
    keep
}
