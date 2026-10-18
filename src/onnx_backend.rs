//! YOLOv8 detector on ONNX Runtime. Expects an Ultralytics-exported ONNX with output `[1, 4 + nc, anchors]`:
//! xywh boxes (pixels, or normalised to 0..1, which is detected and rescaled) followed by class probabilities.

use anyhow::{anyhow, Result};
use image::DynamicImage;
use ndarray::Array4;
use ort::session::Session;
use ort::value::TensorRef;
use std::path::Path;

use crate::detect::{letterbox, nms, Detection, CLASS_NAMES, INPUT_SIZE, NUM_CLASSES};

pub struct OnnxDetector {
    session: Session,
    pub conf_threshold: f32,
    pub iou_threshold: f32,
}

impl OnnxDetector {
    pub fn load(onnx: impl AsRef<Path>, conf: f32, iou: f32) -> Result<Self> {
        let session = crate::runtime::session_builder()?
            .commit_from_file(onnx.as_ref())
            .map_err(|e| anyhow!("loading {}: {e}", onnx.as_ref().display()))?;
        Ok(Self { session, conf_threshold: conf, iou_threshold: iou })
    }

    /// Detect the board and pieces; boxes are in original-image pixels.
    pub fn detect(&mut self, img: &DynamicImage) -> Result<Vec<Detection>> {
        let t0 = std::time::Instant::now();
        let (canvas, lb) = letterbox(img);
        let t_pre = t0.elapsed();
        let s = INPUT_SIZE;
        // HWC u8 -> CHW f32 in one pass over contiguous memory.
        let plane = s * s;
        let mut chw = vec![0f32; 3 * plane];
        for (i, px) in canvas.as_raw().chunks_exact(3).enumerate() {
            chw[i] = px[0] as f32 / 255.0;
            chw[plane + i] = px[1] as f32 / 255.0;
            chw[2 * plane + i] = px[2] as f32 / 255.0;
        }
        let input = Array4::from_shape_vec((1, 3, s, s), chw)?;
        let tensor = TensorRef::from_array_view(input.view()).map_err(|e| anyhow!("{e}"))?;
        let t1 = std::time::Instant::now();
        let outputs = self.session.run(ort::inputs!["images" => tensor]).map_err(|e| anyhow!("inference: {e}"))?;
        let (shape, data) = outputs[0].try_extract_tensor::<f32>().map_err(|e| anyhow!("{e}"))?;
        let t_run = t1.elapsed();
        let n = shape[2] as usize;
        // Some exports (e.g. the one on Hugging Face) emit boxes normalised to 0..1 instead of pixels.
        let box_scale = if data[..4 * n].iter().fold(0f32, |m, v| m.max(v.abs())) <= 1.5 { INPUT_SIZE as f32 } else { 1.0 };
        let mut cands = Vec::new();
        for i in 0..n {
            let (class, conf) = (0..NUM_CLASSES)
                .map(|c| (c, data[(4 + c) * n + i]))
                .fold((0, f32::MIN), |a, b| if b.1 > a.1 { b } else { a });
            if conf < self.conf_threshold {
                continue;
            }
            let (cx, cy, w, h) = (data[i] * box_scale, data[n + i] * box_scale, data[2 * n + i] * box_scale, data[3 * n + i] * box_scale);
            let (x1, y1, x2, y2) = lb.unmap(cx - w / 2.0, cy - h / 2.0, cx + w / 2.0, cy + h / 2.0);
            cands.push(Detection { class, label: CLASS_NAMES[class], confidence: conf, x1, y1, x2, y2 });
        }
        if std::env::var_os("CHESS_VISION_DEBUG").is_some() {
            eprintln!("letterbox {t_pre:?}, ort run {t_run:?}, total {:?}", t0.elapsed());
        }
        Ok(nms(cands, self.iou_threshold))
    }
}
