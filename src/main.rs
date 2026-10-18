use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};
use chess_vision::board::{Orientation, Position, Side};
use chess_vision::output::{Format, Report};
use chess_vision::render::{PieceColors, Pieces, RenderOptions, Style, Theme};
use chess_vision::OnnxDetector;
use clap::{Parser, ValueEnum};

#[derive(Clone, Copy, ValueEnum)]
enum FormatArg {
    Pretty,
    #[value(alias = "ascii")]
    Board,
    Fen,
    Links,
    Json,
}

#[derive(Clone, Copy, ValueEnum)]
enum StyleArg {
    Art,
    Frame,
    Ascii,
    Minimal,
}

#[derive(Clone, Copy, ValueEnum)]
enum ThemeArg {
    Dark,
    Light,
}

#[derive(Clone, Copy, ValueEnum)]
enum PiecesArg {
    Unicode,
    Solid,
    Letters,
}

#[derive(Clone, Copy, ValueEnum)]
enum ColorArg {
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, ValueEnum)]
enum TurnArg {
    #[value(alias = "w")]
    White,
    #[value(alias = "b")]
    Black,
}

#[derive(Clone, Copy, ValueEnum)]
enum OrientationArg {
    Auto,
    White,
    Black,
}

/// Recognise a chess position from an image.
#[derive(Parser)]
#[command(version)]
struct Args {
    /// Image file (png/jpg/bmp/webp); omit or use `-` to read the image from stdin
    image: Option<PathBuf>,
    /// YOLO ONNX model (default: downloaded to the cache). Accepts pixel-space or 0-1 normalised boxes.
    #[arg(long, env = "CHESS_VISION_ONNX")]
    onnx: Option<PathBuf>,
    /// Output format
    #[arg(short, long, value_enum, default_value = "pretty")]
    format: FormatArg,
    /// Board style
    #[arg(short, long, value_enum, default_value = "art")]
    style: StyleArg,
    /// Colours for your terminal background: dark (default) or light (light-grey squares, darkened pieces)
    #[arg(long, value_enum, default_value = "dark")]
    theme: ThemeArg,
    /// Piece set: unicode = emoji-free outline glyphs (default; letters for ascii/minimal), solid = filled glyphs, letters
    #[arg(long, value_enum)]
    pieces: Option<PiecesArg>,
    /// ANSI colours: auto = only on a terminal and when NO_COLOR is unset
    #[arg(long, value_enum, default_value = "auto")]
    color: ColorArg,
    /// Characters per square for frame/ascii/minimal (1-9)
    #[arg(long, value_name = "N")]
    cell_width: Option<usize>,
    /// Piece colours: white-grey (default), white-blue, gold-blue, red-blue, green-purple, or RRGGBB,RRGGBB (white side first)
    #[arg(long, value_parser = PieceColors::parse, default_value = "white-grey", value_name = "COLORS")]
    piece_colors: PieceColors,
    /// Side to move, if known (default: show both)
    #[arg(short, long, value_enum)]
    turn: Option<TurnArg>,
    /// Use PaddleOCR files from this directory (en_rec.onnx, en_dict.txt) instead of the cache
    #[arg(long, env = "CHESS_VISION_OCR")]
    ocr_dir: Option<PathBuf>,
    /// Never download models; fail (or skip OCR) if they are not cached
    #[arg(long)]
    no_download: bool,
    /// Download all models into the cache (~/.cache/chess-vision) and exit
    #[arg(long)]
    download_models: bool,
    /// Which colour is at the bottom of the image (auto: read the board's coordinate labels with
    /// PaddleOCR if available, else guess from piece positions)
    #[arg(long, value_enum, default_value = "auto")]
    orientation: OrientationArg,
    /// Castling rights for the FEN, e.g. KQkq or - (default: inferred from piece placement)
    #[arg(long)]
    castling: Option<String>,
    /// Minimum detection confidence
    #[arg(long, default_value_t = 0.25)]
    conf: f32,
    /// NMS IoU threshold
    #[arg(long, default_value_t = 0.45)]
    iou: f32,
    /// Repeat detection N times (after one warm-up) and print mean/min ms to stderr
    #[arg(long, value_name = "N")]
    bench: Option<usize>,
    /// Print timing to stderr
    #[arg(long)]
    timing: bool,
}

fn read_label_orientation(
    img: &image::DynamicImage,
    bb: (f32, f32, f32, f32),
    dir: Option<&std::path::Path>,
    allow_download: bool,
) -> Option<Orientation> {
    let files = match dir {
        Some(d) => Ok((d.join("en_rec.onnx"), d.join("en_dict.txt"))),
        None => chess_vision::models::ocr_files(allow_download),
    };
    let result = files.and_then(|(model, dict)| {
        let mut ocr = chess_vision::ocr::Recognizer::load(model, dict)?;
        chess_vision::labels::orientation_from_labels(img, bb, &mut ocr)
    });
    match result {
        Ok(o) => o,
        Err(e) => {
            eprintln!("warning: could not read board labels ({e:#}); using piece positions");
            None
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();
    if args.download_models {
        for path in chess_vision::models::download_all()? {
            eprintln!("cached: {}", path.display());
        }
        return Ok(());
    }
    let img = match args.image.as_deref().filter(|p| p.as_os_str() != "-") {
        Some(path) => std::fs::read(path).with_context(|| format!("reading {}", path.display()))?,
        None => {
            use std::io::{IsTerminal, Read};
            if std::io::stdin().is_terminal() {
                bail!("no image given: pass a file path, or pipe an image into stdin");
            }
            let mut buf = Vec::new();
            std::io::stdin().lock().read_to_end(&mut buf).context("reading image from stdin")?;
            buf
        }
    };
    // The format is guessed from the content, not the extension.
    let img = image::load_from_memory(&img).context("decoding image")?;
    let t0 = Instant::now();
    let onnx = match &args.onnx {
        Some(p) => p.clone(),
        None => chess_vision::models::yolo_onnx(!args.no_download)?,
    };
    let mut detector = OnnxDetector::load(onnx, args.conf, args.iou)?;
    let t1 = Instant::now();
    let dets = detector.detect(&img)?;
    if let Some(n) = args.bench {
        let ms: Vec<f64> = (0..n)
            .map(|_| {
                let t = Instant::now();
                detector.detect(&img).map(|_| t.elapsed().as_secs_f64() * 1e3)
            })
            .collect::<Result<_, _>>()?;
        let mean = ms.iter().sum::<f64>() / n as f64;
        eprintln!("end-to-end: mean {mean:.1} ms  min {:.1} ms ({n} runs)", ms.iter().cloned().fold(f64::MAX, f64::min));
    }
    if args.timing {
        eprintln!("load: {:?}, detect: {:?}", t1 - t0, t1.elapsed());
    }

    let orientation = match args.orientation {
        OrientationArg::Auto => Orientation::Auto,
        OrientationArg::White => Orientation::WhiteBottom,
        OrientationArg::Black => Orientation::BlackBottom,
    };
    // Auto: trust printed coordinates when readable (PaddleOCR), else fall back to piece positions.
    let (orientation, source) = match (orientation, chess_vision::board::board_box(&dets)) {
        (Orientation::Auto, Some(bb)) => match read_label_orientation(&img, bb, args.ocr_dir.as_deref(), !args.no_download) {
            Some(o) => (o, "board labels"),
            None => (Orientation::Auto, "piece positions"),
        },
        (Orientation::Auto, None) => (Orientation::Auto, "piece positions"),
        (o, _) => (o, "--orientation"),
    };
    if args.timing {
        eprintln!("orientation from: {source}");
    }
    let position = Position::from_detections(&dets, orientation);
    let color = match args.color {
        ColorArg::Always => true,
        ColorArg::Never => false,
        ColorArg::Auto => {
            use std::io::IsTerminal;
            std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none_or(|v| v.is_empty())
        }
    };
    let render = RenderOptions {
        style: match args.style {
            StyleArg::Art => Style::Art,
            StyleArg::Frame => Style::Frame,
            StyleArg::Ascii => Style::Ascii,
            StyleArg::Minimal => Style::Minimal,
        },
        theme: match args.theme {
            ThemeArg::Dark => Theme::Dark,
            ThemeArg::Light => Theme::Light,
        },
        pieces: args.pieces.map(|p| match p {
            PiecesArg::Unicode => Pieces::Unicode,
            PiecesArg::Solid => Pieces::Solid,
            PiecesArg::Letters => Pieces::Letters,
        }),
        color,
        cell_width: args.cell_width,
        piece_colors: args.piece_colors,
    };
    let report = Report {
        render,
        castling: args.castling.unwrap_or_else(|| position.inferred_castling()),
        position: &position,
        detections: &dets,
        turn: args.turn.map(|t| if matches!(t, TurnArg::White) { Side::White } else { Side::Black }),
    };
    let format = match args.format {
        FormatArg::Pretty => Format::Pretty,
        FormatArg::Board => Format::Board,
        FormatArg::Fen => Format::Fen,
        FormatArg::Links => Format::Links,
        FormatArg::Json => Format::Json,
    };
    print!("{}", report.render(format));
    Ok(())
}
