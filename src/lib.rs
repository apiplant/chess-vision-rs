//! Chess position recognition from images: YOLOv8m detection on ONNX Runtime -> squares -> FEN.

pub mod board;
pub mod detect;
pub mod labels;
pub mod models;
pub mod ocr;
pub mod onnx_backend;
pub mod output;
pub mod render;
pub mod runtime;

pub use board::{Orientation, Position, Side};
pub use detect::Detection;
pub use onnx_backend::OnnxDetector;
