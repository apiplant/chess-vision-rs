# chess-vision

Chess position from an image: YOLOv8m piece/board detection on ONNX Runtime -> squares -> FEN, ASCII board and Lichess links.

```
chess-vision shot.png                 # ASCII board + Lichess links (white / black to move) + FEN
cat shot.png | chess-vision        # or `-`: image from stdin, e.g. `wl-paste | chess-vision`, `grim -g "$(slurp)" - | chess-vision`
chess-vision shot.png -t b            # black to move: one link
chess-vision shot.png -f fen          # pretty | ascii | fen | links | json
chess-vision shot.png --orientation black --castling KQkq
```

## Install

Prebuilt packages for macOS (Apple Silicon), Linux x86_64 and Linux arm64 (Arch: x86_64 only).

**Homebrew** (macOS, Linux):

```bash
brew tap apiplant/tap
brew install apiplant/tap/chess-vision-rs
```

**Debian/Ubuntu**, via the signed apt repository (one-time setup, then `apt upgrade` picks up releases):

```bash
curl -sSfL https://apt.apiplant.com/apiplant-archive-keyring.gpg | sudo tee /usr/share/keyrings/apiplant.gpg > /dev/null
echo "deb [signed-by=/usr/share/keyrings/apiplant.gpg] https://apt.apiplant.com stable main" | sudo tee /etc/apt/sources.list.d/apiplant.list > /dev/null
sudo apt update && sudo apt install chess-vision-rs
```

**Arch Linux**, via the signed pacman repository (one-time setup, then `pacman -Syu` picks up releases):

```bash
curl -sSfL https://apiplant.github.io/pacman/apiplant.gpg -o /tmp/apiplant.gpg
keyid=$(gpg --show-keys --with-colons /tmp/apiplant.gpg | awk -F: '/^pub:/ { print $5; exit }') && sudo pacman-key --add /tmp/apiplant.gpg && sudo pacman-key --finger "$keyid" && sudo pacman-key --lsign-key "$keyid"
printf '\n[apiplant]\nSigLevel = Required DatabaseOptional\nServer = https://apiplant.github.io/pacman/$arch\n' | sudo tee -a /etc/pacman.conf > /dev/null
sudo pacman -Sy chess-vision-rs
```

**Single package, no repository:**

```bash
sudo dpkg -i chess-vision-rs_*_amd64.deb
sudo pacman -U chess-vision-rs-*-x86_64.pkg.tar.zst
```

**Plain archive:** download and unpack the one for your platform from the [releases page](https://github.com/apiplant/chess-vision-rs/releases).

**From source:** `cargo install --path .` (or `cargo build --release`).

The models (~112 MB) are downloaded to `~/.cache/chess-vision` on first run; see Setup below. Release process: [`packaging/README.md`](packaging/README.md).

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
