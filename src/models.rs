//! Model files: resolved from the XDG cache (`$XDG_CACHE_HOME/chess-vision`, default `~/.cache/chess-vision`)
//! and downloaded from Hugging Face on first use. Revisions are pinned.

use anyhow::{anyhow, bail, Context, Result};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

struct Remote {
    /// Path under the cache directory.
    rel: &'static str,
    url: &'static str,
}

/// Chess piece + board YOLOv8m (AGPL-3.0), NAKSTStudio/yolov8m-chess-piece-detection @ 3b2c734.
const YOLO: Remote = Remote {
    rel: "yolov8m-chess/3b2c734/best.onnx",
    url: "https://huggingface.co/NAKSTStudio/yolov8m-chess-piece-detection/resolve/3b2c734aeade4646ea313da333ee670a4869c46f/best.onnx",
};
/// PaddleOCR PP-OCRv5 English mobile recognizer + dictionary (Apache-2.0), monkt/paddleocr-onnx @ 7b02d0a.
const OCR_MODEL: Remote = Remote {
    rel: "paddleocr-en/7b02d0a/rec.onnx",
    url: "https://huggingface.co/monkt/paddleocr-onnx/resolve/7b02d0a30a07ba2b92ad1ff5a8941ae2c633de65/languages/english/rec.onnx",
};
const OCR_DICT: Remote = Remote {
    rel: "paddleocr-en/7b02d0a/dict.txt",
    url: "https://huggingface.co/monkt/paddleocr-onnx/resolve/7b02d0a30a07ba2b92ad1ff5a8941ae2c633de65/languages/english/dict.txt",
};

pub fn cache_dir() -> PathBuf {
    let base = std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .unwrap_or_else(|| std::env::temp_dir());
    base.join("chess-vision")
}

/// Path of a cached model, downloading it first if missing (unless `allow_download` is false).
fn ensure(remote: &Remote, allow_download: bool) -> Result<PathBuf> {
    let dest = cache_dir().join(remote.rel);
    if dest.exists() {
        return Ok(dest);
    }
    if !allow_download {
        bail!("{} is not cached and downloads are disabled (--no-download)", dest.display());
    }
    download(remote.url, &dest).with_context(|| format!("downloading {}", remote.url))?;
    Ok(dest)
}

fn download(url: &str, dest: &Path) -> Result<()> {
    fs::create_dir_all(dest.parent().ok_or_else(|| anyhow!("bad cache path"))?)?;
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(900))).build().into();
    let resp = agent.get(url).call().map_err(|e| anyhow!("{e}"))?;
    let expected: Option<u64> = resp.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok());
    let name = dest.file_name().and_then(|n| n.to_str()).unwrap_or("model");
    match expected {
        Some(n) => eprintln!("chess-vision: downloading {name} ({:.1} MB) to {}", n as f64 / 1e6, dest.display()),
        None => eprintln!("chess-vision: downloading {name} to {}", dest.display()),
    }

    // Write to a temp file and rename, so an interrupted download never leaves a truncated model behind.
    let part = dest.with_extension("part");
    let mut out = File::create(&part)?;
    let mut body = resp.into_body().into_reader();
    let (mut buf, mut total, mut next_report) = (vec![0u8; 1 << 16], 0u64, 10u64 << 20);
    loop {
        let n = body.read(&mut buf)?;
        if n == 0 {
            break;
        }
        out.write_all(&buf[..n])?;
        total += n as u64;
        if expected.is_some_and(|e| e > (20 << 20)) && total >= next_report {
            eprint!("\r  {:3.0}%", total as f64 * 100.0 / expected.unwrap() as f64);
            next_report += 10 << 20;
        }
    }
    out.flush()?;
    drop(out);
    if let Some(e) = expected {
        if total != e {
            let _ = fs::remove_file(&part);
            bail!("incomplete download: got {total} of {e} bytes");
        }
    }
    fs::rename(&part, dest)?;
    if expected.is_some_and(|e| e > (20 << 20)) {
        eprintln!("\r  100%");
    }
    Ok(())
}

pub fn yolo_onnx(allow_download: bool) -> Result<PathBuf> {
    ensure(&YOLO, allow_download)
}

/// (recognizer model, dictionary) of the PaddleOCR English recognizer.
pub fn ocr_files(allow_download: bool) -> Result<(PathBuf, PathBuf)> {
    Ok((ensure(&OCR_MODEL, allow_download)?, ensure(&OCR_DICT, allow_download)?))
}

/// Fetch everything now (for `--download-models`).
pub fn download_all() -> Result<Vec<PathBuf>> {
    let (m, d) = ocr_files(true)?;
    Ok(vec![yolo_onnx(true)?, m, d])
}
