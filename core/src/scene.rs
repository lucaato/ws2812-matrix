use crate::composition::{Chain2dIter, Composite2dIter, Position};
use crate::glyph::{GraphicIter3by5, GraphicIter5by5};
use crate::Color;

/// Demo scene shared by the firmware and the emulator: the letters
/// `T`, `E`, `A` chained next to each other, plus an `M`, bouncing
/// vertically on a 16x16 matrix.
pub type TeamMatrix =
    Composite2dIter<Position<Chain2dIter<3, GraphicIter3by5>>, Position<GraphicIter5by5>, Color>;

/// Font: 3x5 glyph `T`.
pub const GLYPH_T: u16 = 0b111_010_010_010_010;
/// Font: 3x5 glyph `E`.
pub const GLYPH_E: u16 = 0b111_100_110_100_111;
/// Font: 3x5 glyph `A`.
pub const GLYPH_A: u16 = 0b010_101_111_101_101;
/// Font: 5x5 glyph `M`.
pub const GLYPH_M: u32 = 0b10001_11011_10101_10001_10001;

/// The animation travels this many cells while bouncing, keeping one row
/// of margin at the bottom of a 16x16 matrix.
const TRAVEL: usize = 16 - 5;

/// Builds the demo scene at animation step `t`.
///
/// Steps are meant to be driven by a periodic timer (the firmware uses one
/// step per second); every iteration, the same `t` produces the same frame
/// on both the emulator and the matrix.
#[must_use]
pub fn team_matrix(t: usize) -> TeamMatrix {
    let y = y_offset(t);

    let letters = Chain2dIter::new([
        GraphicIter3by5::with_on_color(GLYPH_T, Color::DARK_GREEN),
        GraphicIter3by5::with_on_color(GLYPH_E, Color::DARK_BLUE),
        GraphicIter3by5::with_on_color(GLYPH_A, Color::DARK_GREEN),
    ])
    .expect("all chained glyphs share the same height");

    Composite2dIter::new(
        16,
        16,
        Position::new(letters, 1, y),
        Position::new(
            GraphicIter5by5::with_on_color(GLYPH_M, Color::DARK_BLUE),
            10,
            y,
        ),
        Color::BLACK,
    )
}

/// Vertical offset of the text at animation step `t`: a triangle wave that
/// scrolls the text down, bounces at the bottom margin, scrolls back up
/// and bounces at the top.
#[must_use]
pub fn y_offset(t: usize) -> usize {
    let phase = t % (2 * TRAVEL);

    if phase <= TRAVEL {
        phase
    } else {
        2 * TRAVEL - phase
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::*;

    #[test]
    fn bounces_within_matrix() {
        for t in 0..2 * TRAVEL * 5 {
            assert!(y_offset(t) < 16);
        }
    }

    #[test]
    fn triangle_wave_reaches_top_and_bottom_margins() {
        assert_eq!(y_offset(0), 0);
        assert_eq!(y_offset(TRAVEL), TRAVEL);
        assert_eq!(y_offset(2 * TRAVEL), 0);
        assert_eq!(y_offset(2 * TRAVEL + 1), 1);
    }

    /// The whole scene streams exactly one 16x16 frame per step.
    #[test]
    fn scene_streams_256_cells() {
        for t in 0..2 * TRAVEL + 2 {
            let scene = team_matrix(t);

            assert_eq!(scene.size_hint(), (256, Some(256)));

            let frame: Vec<Color> = scene.clone().collect();

            assert_eq!(frame.len(), 256);
        }
    }
}
