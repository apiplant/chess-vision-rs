//! The process-wide ONNX Runtime environment (ort allows only one), shared by the detector and OCR.

use anyhow::{anyhow, Result};
use ort::environment::Environment;
use ort::session::Session;
use std::sync::Mutex;

static ENV: Mutex<Option<Environment>> = Mutex::new(None);

pub fn env() -> Result<Environment> {
    let mut env = ENV.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if env.is_none() {
        *env = Some(ort::init().build().map_err(|e| anyhow!("onnx runtime init: {e}"))?);
    }
    Ok(env.clone().expect("just initialised"))
}

/// Session builder tuned for CPU inference (the settings Ultralytics' own Rust runner uses): full graph
/// optimisation, all cores for intra-op work, one inter-op thread, memory-pattern reuse.
/// Override the thread count with `CHESS_VISION_THREADS`.
pub fn session_builder() -> Result<ort::session::builder::SessionBuilder> {
    use ort::session::builder::GraphOptimizationLevel;
    let threads = std::env::var("CHESS_VISION_THREADS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| std::thread::available_parallelism().map_or(4, |n| n.get()));
    Session::builder(&env()?)
        .map_err(err)?
        .with_optimization_level(GraphOptimizationLevel::All)
        .map_err(err)?
        .with_intra_threads(threads)
        .map_err(err)?
        .with_inter_threads(1)
        .map_err(err)?
        .with_memory_pattern(true)
        .map_err(err)
}

fn err<T>(e: ort::Error<T>) -> anyhow::Error {
    anyhow!("{e}")
}
