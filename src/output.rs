//! Output formats for a recognised position.

use serde::Serialize;

use crate::board::{lichess_url, Position, Side};
use crate::detect::Detection;
use crate::render::RenderOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// ASCII board + Lichess links (default).
    Pretty,
    /// Board only.
    Board,
    /// FEN only (needs a side to move; defaults to white).
    Fen,
    /// Lichess link(s) only.
    Links,
    /// Everything, as JSON (includes raw detections).
    Json,
}

pub struct Report<'a> {
    pub position: &'a Position,
    pub detections: &'a [Detection],
    /// `Some` if the caller knows who moves; `None` shows both options.
    pub turn: Option<Side>,
    pub castling: String,
    pub render: RenderOptions,
}

#[derive(Serialize)]
struct Link {
    turn: Side,
    fen: String,
    lichess: String,
}

impl Report<'_> {
    fn links(&self) -> Vec<Link> {
        let sides: &[Side] = match self.turn {
            Some(Side::White) => &[Side::White],
            Some(Side::Black) => &[Side::Black],
            None => &[Side::White, Side::Black],
        };
        sides
            .iter()
            .map(|&turn| {
                let fen = self.position.fen(turn, &self.castling);
                Link { turn, lichess: lichess_url(&fen), fen }
            })
            .collect()
    }

    pub fn render(&self, format: Format) -> String {
        let links = self.links();
        let label = |s: Side| if s == Side::White { "White" } else { "Black" };
        match format {
            Format::Board => self.position.render(&self.render),
            Format::Fen => format!("{}\n", self.position.fen(self.turn.unwrap_or(Side::White), &self.castling)),
            Format::Links => links.iter().map(|l| format!("{}\n", l.lichess)).collect(),
            Format::Pretty => {
                let mut s = self.position.render(&self.render);
                if !self.position.board_detected {
                    s.push_str("\n(no board box detected; squares estimated from the pieces' extent)\n");
                }
                s.push('\n');
                for l in &links {
                    s.push_str(&format!("{} to move: {}\n", label(l.turn), l.lichess));
                }
                s.push_str(&format!("\nFEN: {}\n", links[0].fen));
                s
            }
            Format::Json => {
                #[derive(Serialize)]
                struct Out<'a> {
                    board_detected: bool,
                    piece_count: usize,
                    placement: String,
                    ascii: String,
                    positions: Vec<Link>,
                    detections: &'a [Detection],
                }
                let out = Out {
                    board_detected: self.position.board_detected,
                    piece_count: self.position.piece_count(),
                    placement: self.position.piece_placement(),
                    ascii: self.position.ascii(),
                    positions: links,
                    detections: self.detections,
                };
                serde_json::to_string_pretty(&out).unwrap() + "\n"
            }
        }
    }
}
