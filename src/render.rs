//! Board rendering for the terminal: four styles, two themes, configurable piece colours.

type Grid = [[Option<char>; 8]; 8];

/// Layout of the rendered board.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    /// Big 9x4 squares, no lines, with hand-drawn ASCII-art pieces (white hollow, black filled);
    /// squares alternate dark grey / pure black when colour is on (default).
    Art,
    /// Compact: a rounded outline, one centred glyph per 3x1 cell; cells alternate dark grey / pure
    /// black when colour is on.
    Frame,
    /// Full grid of joined box-drawing lines around every square, with FEN letters.
    Ascii,
    /// Plain text diagram: letters and dots, no borders (coloured letters when colour is on).
    Minimal,
}

/// Colouring matched to the terminal's background.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Theme {
    /// Dark terminal: dark squares, pieces as given by the piece colours.
    Dark,
    /// Light terminal: light-grey squares, pieces darkened.
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pieces {
    /// Outline chess glyphs (U+2654..U+2659) for both sides, told apart by colour. These have no emoji
    /// form, so they never turn into emoji.
    Unicode,
    /// Solid glyphs (U+265A..U+265F) for both sides. The solid pawn is an emoji in many fonts.
    Solid,
    /// FEN letters: uppercase white, lowercase black.
    Letters,
}

/// Foreground colours (RGB) of the two sides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PieceColors {
    pub white: (u8, u8, u8),
    pub black: (u8, u8, u8),
}

impl PieceColors {
    pub const WHITE_BLUE: Self = Self { white: (255, 255, 255), black: (45, 90, 235) };
    pub const GOLD_BLUE: Self = Self { white: (255, 214, 102), black: (102, 178, 255) };
    pub const RED_BLUE: Self = Self { white: (255, 112, 100), black: (100, 170, 255) };
    pub const GREEN_PURPLE: Self = Self { white: (140, 230, 120), black: (200, 140, 255) };
    pub const WHITE_GREY: Self = Self { white: (255, 255, 255), black: (125, 125, 125) };

    /// A preset name (`white-blue`, `gold-blue`, `red-blue`, `green-purple`, `white-grey`) or two hex
    /// colours `RRGGBB,RRGGBB` (white side first).
    pub fn parse(s: &str) -> Result<Self, String> {
        match s {
            "white-blue" => return Ok(Self::WHITE_BLUE),
            "gold-blue" => return Ok(Self::GOLD_BLUE),
            "red-blue" => return Ok(Self::RED_BLUE),
            "green-purple" => return Ok(Self::GREEN_PURPLE),
            "white-grey" => return Ok(Self::WHITE_GREY),
            _ => {}
        }
        let hex = |h: &str| -> Result<(u8, u8, u8), String> {
            let h = h.trim().trim_start_matches('#');
            let v = (h.len() == 6).then(|| u32::from_str_radix(h, 16).ok()).flatten();
            v.map(|v| ((v >> 16) as u8, (v >> 8) as u8, v as u8)).ok_or_else(|| format!("`{h}` is not a RRGGBB colour"))
        };
        match s.split_once(',') {
            Some((w, b)) => Ok(Self { white: hex(w)?, black: hex(b)? }),
            None => Err(format!(
                "`{s}`: expected a preset (white-blue, gold-blue, red-blue, green-purple, white-grey) or RRGGBB,RRGGBB"
            )),
        }
    }

    /// Ink for a piece; darkened on the light theme so it reads on light squares.
    fn ink(self, white: bool, theme: Theme) -> (u8, u8, u8) {
        let (r, g, b) = if white { self.white } else { self.black };
        match theme {
            Theme::Dark => (r, g, b),
            Theme::Light => ((r as f32 * 0.55) as u8, (g as f32 * 0.55) as u8, (b as f32 * 0.55) as u8),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RenderOptions {
    pub style: Style,
    pub theme: Theme,
    /// `None` picks the style's default (letters for ascii/minimal and when colour is off, else outline glyphs).
    pub pieces: Option<Pieces>,
    /// Use ANSI colours (square backgrounds in art/frame, piece colours).
    pub color: bool,
    /// Characters per square for frame/ascii/minimal (art has a fixed size).
    pub cell_width: Option<usize>,
    pub piece_colors: PieceColors,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            style: Style::Art,
            theme: Theme::Dark,
            pieces: None,
            color: true,
            cell_width: None,
            piece_colors: PieceColors::WHITE_GREY,
        }
    }
}

impl RenderOptions {
    /// Plain, colourless ASCII (used for the JSON `ascii` field).
    pub fn plain_ascii() -> Self {
        Self { style: Style::Ascii, color: false, ..Self::default() }
    }
}

impl Style {
    /// (width, height) of one square in characters.
    fn cell(self) -> (usize, usize) {
        match self {
            Style::Art => (9, 4),
            Style::Frame | Style::Ascii => (3, 1),
            Style::Minimal => (2, 1),
        }
    }
}

/// 7x4 ASCII-art sprites; `*` is the body fill (space for white, `#` for black).
fn sprite(piece: char) -> [&'static str; 4] {
    match piece.to_ascii_lowercase() {
        'k' => [r"  _|_  ", r"   |   ", r" (***) ", r" /***\ "],
        'q' => [r" o o o ", r" \***/ ", r"  )*(  ", r" /***\ "],
        'r' => [r" n_n_n ", r" |***| ", r"  |*|  ", r" |___| "],
        'b' => [r"   ^   ", r"  (*\  ", r"  )*(  ", r" /***\ "],
        'n' => [r"  ,^^. ", r" /*o** ", r"  /*/  ", r" /***\ "],
        _ => [r"       ", r"   o   ", r"  (*)  ", r" /***\ "],
    }
}

fn glyph(piece: char, set: Pieces) -> String {
    let solid = match set {
        Pieces::Letters => return piece.to_string(),
        Pieces::Solid => true,
        // Colour tells the sides apart, so the outline glyphs serve both.
        Pieces::Unicode => false,
    };
    // U+FE0E asks for text (not emoji) presentation of the solid pawn, though not every terminal honours it.
    match (piece.to_ascii_lowercase(), solid) {
        ('k', true) => "♚",
        ('q', true) => "♛",
        ('r', true) => "♜",
        ('b', true) => "♝",
        ('n', true) => "♞",
        ('p', true) => "♟\u{fe0e}",
        ('k', false) => "♔",
        ('q', false) => "♕",
        ('r', false) => "♖",
        ('b', false) => "♗",
        ('n', false) => "♘",
        _ => "♙",
    }
    .to_string()
}

/// Square background: dark grey / pure black (light greys on the light theme).
fn square_rgb(theme: Theme, light: bool) -> (u8, u8, u8) {
    match (theme, light) {
        (Theme::Dark, true) => (56, 56, 56),
        (Theme::Dark, false) => (0, 0, 0),
        (Theme::Light, true) => (236, 236, 236),
        (Theme::Light, false) => (212, 212, 212),
    }
}

const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";

fn fg((r, g, b): (u8, u8, u8)) -> String {
    format!("\x1b[38;2;{r};{g};{b}m")
}
fn bg((r, g, b): (u8, u8, u8)) -> String {
    format!("\x1b[48;2;{r};{g};{b}m")
}

pub fn render(grid: &Grid, opts: &RenderOptions) -> String {
    let style = opts.style;
    let color = opts.color;
    let (mut cw, ch) = style.cell();
    if style != Style::Art {
        if let Some(w) = opts.cell_width {
            cw = w.clamp(1, 9);
        }
    }
    // Glyphs only when colour can tell the sides apart (the outline glyphs are the same for both).
    let set = opts.pieces.unwrap_or(match style {
        Style::Frame if color => Pieces::Unicode,
        _ => Pieces::Letters,
    });
    let dim = |s: &str| if color { format!("{DIM}{s}{RESET}") } else { s.to_string() };

    // One line of one square. The piece (or sprite line) is centred, left-biased when the width is even.
    let cell = |r: usize, c: usize, line: usize| -> String {
        let light = (r + c) % 2 == 0;
        let piece = grid[r][c];
        let (body, width) = match (style, piece) {
            (Style::Art, Some(p)) => {
                let fill = if p.is_ascii_uppercase() { " " } else { "#" };
                (sprite(p)[line].replace('*', fill), 7)
            }
            (Style::Art, None) => {
                // Without colour, empty dark squares get a faint dot so the checkerboard stays readable.
                let dot = !color && !light && line == ch / 2;
                (if dot { "   .   " } else { "       " }.to_string(), 7)
            }
            (Style::Minimal, None) => ("·".into(), 1),
            (_, Some(p)) => (glyph(p, set), 1),
            (_, None) => (" ".into(), 1),
        };
        let left = (cw - width) / 2;
        let text = format!("{}{body}{}", " ".repeat(left), " ".repeat(cw - width - left));
        if !color {
            return text;
        }
        let ink = piece.map(|p| opts.piece_colors.ink(p.is_ascii_uppercase(), opts.theme));
        let bold = if piece.is_some() { "\x1b[1m" } else { "" };
        let ink = ink.map(fg).unwrap_or_default();
        match style {
            // Squares are filled with colour.
            Style::Art | Style::Frame => {
                format!("{}{ink}{bold}{text}{RESET}", bg(square_rgb(opts.theme, light)))
            }
            // Ascii / minimal: coloured letters on the terminal's own background, dimmed dots.
            _ if piece.is_some() => format!("{ink}{bold}{text}{RESET}"),
            _ => dim(&text),
        }
    };

    let files = |indent: usize, gap: usize| {
        let mut s = " ".repeat(indent);
        for c in 0..8 {
            let left = (cw - 1) / 2;
            s.push_str(&" ".repeat(left));
            s.push((b'a' + c as u8) as char);
            s.push_str(&" ".repeat(cw - 1 - left + gap));
        }
        dim(s.trim_end())
    };
    let rank_label = |r: usize, line: usize| dim(&if line == ch / 2 { (8 - r).to_string() } else { " ".into() });

    let mut out = String::new();
    match style {
        Style::Art | Style::Minimal => {
            for r in 0..8 {
                for line in 0..ch {
                    let cells: Vec<String> = (0..8).map(|c| cell(r, c, line)).collect();
                    out.push_str(&format!("{} {}\n", rank_label(r, line), cells.concat().trim_end()));
                }
            }
            out.push_str(&files(2, 0));
        }
        Style::Frame => {
            let rule = |l: &str, r: &str| dim(&format!("  {l}{}{r}", "─".repeat(8 * cw)));
            out.push_str(&rule("╭", "╮"));
            out.push('\n');
            for r in 0..8 {
                let cells: Vec<String> = (0..8).map(|c| cell(r, c, 0)).collect();
                let v = dim("│");
                out.push_str(&format!("{} {v}{}{v}\n", rank_label(r, 0), cells.concat()));
            }
            out.push_str(&rule("╰", "╯"));
            out.push('\n');
            out.push_str(&files(3, 0));
        }
        Style::Ascii => {
            let rule = |l: &str, m: &str, r: &str| {
                dim(&format!("  {l}{}{r}", vec!["─".repeat(cw); 8].join(m)))
            };
            out.push_str(&rule("┌", "┬", "┐"));
            out.push('\n');
            for r in 0..8 {
                let cells: Vec<String> = (0..8).map(|c| cell(r, c, 0)).collect();
                let v = dim("│");
                out.push_str(&format!("{} {v}{}{v}\n", rank_label(r, 0), cells.join(&v)));
                out.push_str(&if r == 7 { rule("└", "┴", "┘") } else { rule("├", "┼", "┤") });
                out.push('\n');
            }
            out.push_str(&files(3, 1));
        }
    }
    out.push('\n');
    out
}

#[cfg(test)]
mod tests {
    use super::sprite;

    #[test]
    fn sprites_are_7x4() {
        for p in "kqrbnp".chars() {
            for line in sprite(p) {
                assert_eq!(line.chars().count(), 7, "{p}: {line:?}");
            }
        }
    }
}
