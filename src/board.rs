//! Turn detections into a board position: squares, FEN, ASCII art and Lichess links.

use serde::Serialize;

use crate::detect::Detection;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Side {
    White,
    Black,
}

impl Side {
    pub fn fen_char(self) -> char {
        match self {
            Side::White => 'w',
            Side::Black => 'b',
        }
    }
}

/// Which colour is at the bottom of the image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    /// Guess from where the white and black pieces sit (white's centroid below black's => white at bottom).
    Auto,
    WhiteBottom,
    BlackBottom,
}

/// 8x8 grid, `grid[rank_index][file_index]` with rank_index 0 = rank 8 (FEN order).
#[derive(Debug, Clone, Serialize)]
pub struct Position {
    pub grid: [[Option<char>; 8]; 8],
    /// Whether a board box was found (otherwise the bounds of the pieces are used).
    pub board_detected: bool,
}

const FEN_CHARS: [char; 12] = ['K', 'Q', 'R', 'B', 'N', 'P', 'k', 'q', 'r', 'b', 'n', 'p'];

/// Bounds of the most confident board detection, if any.
pub fn board_box(dets: &[Detection]) -> Option<(f32, f32, f32, f32)> {
    dets.iter()
        .filter(|d| d.class == 0)
        .max_by(|a, b| a.confidence.total_cmp(&b.confidence))
        .map(|b| (b.x1, b.y1, b.x2, b.y2))
}

impl Position {
    /// Map detections (class 0 = board, 1..=12 = pieces) onto squares.
    pub fn from_detections(dets: &[Detection], orientation: Orientation) -> Self {
        let board = dets.iter().filter(|d| d.class == 0).max_by(|a, b| a.confidence.total_cmp(&b.confidence));
        let pieces: Vec<&Detection> = dets.iter().filter(|d| d.class != 0).collect();

        let (x1, y1, x2, y2) = match board {
            Some(b) => (b.x1, b.y1, b.x2, b.y2),
            None if !pieces.is_empty() => {
                // Fallback: pieces cover at most 7/8 of each axis from centre to centre; pad by half a square.
                let fold = |f: fn(&Detection) -> f32, init: f32, g: fn(f32, f32) -> f32| pieces.iter().fold(init, |a, d| g(a, f(d)));
                let (l, t) = (fold(|d| d.x1, f32::MAX, f32::min), fold(|d| d.y1, f32::MAX, f32::min));
                let (r, b) = (fold(|d| d.x2, f32::MIN, f32::max), fold(|d| d.y2, f32::MIN, f32::max));
                (l, t, r, b)
            }
            None => return Self { grid: [[None; 8]; 8], board_detected: false },
        };
        let (sw, sh) = ((x2 - x1) / 8.0, (y2 - y1) / 8.0);

        let mut best: [[f32; 8]; 8] = [[0.0; 8]; 8];
        let mut grid = [[None; 8]; 8];
        for p in pieces {
            // Anchor: horizontal centre, 3/4 down the box (robust to tall pieces in perspective).
            let (ax, ay) = ((p.x1 + p.x2) / 2.0, p.y1 + 0.75 * p.h());
            let (col, row) = (((ax - x1) / sw).floor(), ((ay - y1) / sh).floor());
            if !(0.0..8.0).contains(&col) || !(0.0..8.0).contains(&row) {
                continue;
            }
            let (r, c) = (row as usize, col as usize);
            if p.confidence > best[r][c] {
                best[r][c] = p.confidence;
                grid[r][c] = Some(FEN_CHARS[p.class - 1]);
            }
        }
        let black_bottom = match orientation {
            Orientation::BlackBottom => true,
            Orientation::WhiteBottom => false,
            Orientation::Auto => {
                // Mean row (0 = top of image) of each colour; white above black means the board is flipped.
                let mean = |white: bool| {
                    let rows: Vec<f32> = (0..8)
                        .flat_map(|r| (0..8).map(move |c| (r, c)))
                        .filter(|&(r, c)| grid[r][c].is_some_and(|ch| ch.is_ascii_uppercase() == white))
                        .map(|(r, _)| r as f32)
                        .collect();
                    (!rows.is_empty()).then(|| rows.iter().sum::<f32>() / rows.len() as f32)
                };
                matches!((mean(true), mean(false)), (Some(w), Some(b)) if w < b)
            }
        };
        if black_bottom {
            // Rotate 180 degrees so the grid is always in FEN order (rank 8 first, file a first).
            let mut flipped = [[None; 8]; 8];
            for r in 0..8 {
                for c in 0..8 {
                    flipped[7 - r][7 - c] = grid[r][c];
                }
            }
            grid = flipped;
        }
        Self { grid, board_detected: board.is_some() }
    }

    pub fn piece_placement(&self) -> String {
        self.grid
            .iter()
            .map(|rank| {
                let mut s = String::new();
                let mut empty = 0;
                for sq in rank {
                    match sq {
                        Some(c) => {
                            if empty > 0 {
                                s.push_str(&empty.to_string());
                                empty = 0;
                            }
                            s.push(*c);
                        }
                        None => empty += 1,
                    }
                }
                if empty > 0 {
                    s.push_str(&empty.to_string());
                }
                s
            })
            .collect::<Vec<_>>()
            .join("/")
    }

    /// Castling rights implied by king/rook still standing on their home squares.
    pub fn inferred_castling(&self) -> String {
        let at = |r: usize, c: usize| self.grid[r][c];
        let mut s = String::new();
        if at(7, 4) == Some('K') {
            if at(7, 7) == Some('R') {
                s.push('K');
            }
            if at(7, 0) == Some('R') {
                s.push('Q');
            }
        }
        if at(0, 4) == Some('k') {
            if at(0, 7) == Some('r') {
                s.push('k');
            }
            if at(0, 0) == Some('r') {
                s.push('q');
            }
        }
        if s.is_empty() {
            "-".into()
        } else {
            s
        }
    }

    pub fn fen(&self, turn: Side, castling: &str) -> String {
        format!("{} {} {} - 0 1", self.piece_placement(), turn.fen_char(), castling)
    }

    /// Plain colourless ASCII board.
    pub fn ascii(&self) -> String {
        crate::render::render(&self.grid, &crate::render::RenderOptions::plain_ascii())
    }

    pub fn render(&self, opts: &crate::render::RenderOptions) -> String {
        crate::render::render(&self.grid, opts)
    }

    pub fn piece_count(&self) -> usize {
        self.grid.iter().flatten().flatten().count()
    }
}

pub fn lichess_url(fen: &str) -> String {
    format!("https://lichess.org/analysis/{}", fen.replace(' ', "_"))
}
