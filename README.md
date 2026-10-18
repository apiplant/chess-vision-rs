# chess-vision

Chess position from an image: YOLOv8m piece/board detection on ONNX Runtime -> squares -> FEN, ASCII board and Lichess links.

```
chess-vision shot.png                 # ASCII board + Lichess links (white / black to move) + FEN
cat shot.png | chess-vision        # or `-`: image from stdin, e.g. `wl-paste | chess-vision`, `grim -g "$(slurp)" - | chess-vision`
chess-vision shot.png -t b            # black to move: one link
chess-vision shot.png -f fen          # pretty | ascii | fen | links | json
chess-vision shot.png --orientation black --castling KQkq
```

## Rendering

Four styles (`--style`, default `art`); colour is automatic (off when piped or `NO_COLOR` is set), force with `--color always|never`.

| style | look |
|---|---|
| `art` | big 9x4 squares, no lines, ASCII-art pieces (white hollow, black filled with `#`) on alternating dark-grey / pure-black squares |
| `frame` | compact: rounded outline, one centred glyph per 3x1 cell on the same dark-grey / black checkerboard |
| `ascii` | full grid of joined box-drawing lines, FEN letters |
| `minimal` | plain letters and dots |

Pieces are white and dark grey by default (`--piece-colors white-grey|white-blue|gold-blue|red-blue|green-purple|RRGGBB,RRGGBB`, white side first).
`--theme light` swaps to light-grey squares and darker pieces for light terminals. `frame` draws the outline chess glyphs
(`--pieces unicode`; `solid` = filled glyphs, whose pawn is an emoji in many fonts; `letters`); without colour it
falls back to letters. `--cell-width N` sets characters per square for frame/ascii/minimal. `-f board` prints only the board.

## Setup

```
cargo build --release
chess-vision shot.png        # the first run downloads the models (~112 MB) into the cache
chess-vision --download-models   # or fetch them up front
```

Models are cached under `$XDG_CACHE_HOME/chess-vision` (default `~/.cache/chess-vision`), pinned to a Hugging Face revision,
and downloaded to a temp file first, so an interrupted download never leaves a broken model:

| file | source | licence |
|---|---|---|
| `yolov8m-chess/3b2c734/best.onnx` (104 MB) | [NAKSTStudio/yolov8m-chess-piece-detection](https://huggingface.co/NAKSTStudio/yolov8m-chess-piece-detection) | AGPL-3.0 |
| `paddleocr-en/7b02d0a/rec.onnx` + `dict.txt` (7.8 MB) | PP-OCRv5 English recognizer via [monkt/paddleocr-onnx](https://huggingface.co/monkt/paddleocr-onnx) | Apache-2.0 |

`--no-download` fails (YOLO) or skips the label reading (OCR) instead of downloading; `--onnx FILE` and `--ocr-dir DIR`
(`en_rec.onnx`, `en_dict.txt`) use your own files. The Hugging Face ONNX has 0-1 normalised boxes, which are detected and rescaled.

## Orientation (`--orientation auto|white|black`)

`auto` reads the board's printed coordinates (ranks 1-8, files a-h) with PaddleOCR's recognizer and votes on
their direction; if none are readable it compares where white and black pieces sit. `CHESS_VISION_DEBUG=1` shows what was read.

Licences: the NAKST model is AGPL-3.0 (so this project is too); PP-OCRv5 is Apache-2.0.
