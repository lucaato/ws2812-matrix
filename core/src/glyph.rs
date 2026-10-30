use crate::traits::{Buffer2d, Iterator2d};
use crate::Color;

/// Renders a 3x5 bitmap glyph, one color per cell.
///
/// The bitmap is encoded one row at a time, top row first, packed into a
/// `u16` from bit 14 down to bit 0. For example the letter `T` encodes as
/// `0b111_010_010_010_010`.
#[derive(Clone, Debug)]
pub struct GraphicIter3by5 {
    bitmap: u16,
    index: u8,
    on: Color,
    off: Color,
}

impl GraphicIter3by5 {
    pub const WIDTH: usize = 3;
    pub const HEIGHT: usize = 5;
    pub const SIZE: usize = Self::WIDTH * Self::HEIGHT;

    pub fn new(bit_list: u16) -> Self {
        Self::with_on_color(bit_list, Color::DARK_WHITE)
    }

    pub fn with_on_color(bit_list: u16, on: Color) -> Self {
        Self {
            bitmap: bit_list,
            index: Self::SIZE as u8,
            on,
            off: Color::BLACK,
        }
    }
}

impl Iterator2d for GraphicIter3by5 {
    fn get_width(&self) -> usize {
        Self::WIDTH
    }

    fn get_height(&self) -> usize {
        Self::HEIGHT
    }
}

impl Iterator for GraphicIter3by5 {
    type Item = Color;

    fn next(&mut self) -> Option<Self::Item> {
        self.index.checked_sub(1).map(|i| {
            self.index = i;

            if (self.bitmap >> i) & 0x01 > 0 {
                self.on
            } else {
                self.off
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left: usize = self.index.into();
        (left, Some(left))
    }
}

impl Buffer2d for GraphicIter3by5 {
    type Item = <Self as Iterator>::Item;

    fn item_at(&self, x: usize, y: usize) -> Option<Self::Item> {
        self.cell_index(x, y).map(|i| {
            let on = (self.bitmap >> (Self::SIZE - 1 - i)) & 0x01 > 0;

            if on {
                self.on
            } else {
                self.off
            }
        })
    }
}

/// Renders a 5x5 bitmap glyph, one color per cell.
///
/// The bitmap is encoded one row at a time, top row first, packed into a
/// `u32` from bit 24 down to bit 0. For example the letter `M` encodes as
/// `0b10001_11011_10101_10001_10001`.
#[derive(Clone, Debug)]
pub struct GraphicIter5by5 {
    bitmap: u32,
    index: u8,
    on: Color,
    off: Color,
}

impl GraphicIter5by5 {
    pub const WIDTH: usize = 5;
    pub const HEIGHT: usize = 5;
    pub const SIZE: usize = Self::WIDTH * Self::HEIGHT;

    pub fn new(bit_list: u32) -> Self {
        Self::with_on_color(bit_list, Color::WHITE)
    }

    pub fn with_on_color(bit_list: u32, on: Color) -> Self {
        Self {
            bitmap: bit_list,
            index: Self::SIZE as u8,
            on,
            off: Color::BLACK,
        }
    }
}

impl Iterator2d for GraphicIter5by5 {
    fn get_width(&self) -> usize {
        Self::WIDTH
    }

    fn get_height(&self) -> usize {
        Self::HEIGHT
    }
}

impl Iterator for GraphicIter5by5 {
    type Item = Color;

    fn next(&mut self) -> Option<Self::Item> {
        self.index.checked_sub(1).map(|i| {
            self.index = i;

            if (self.bitmap >> i) & 0x01 > 0 {
                self.on
            } else {
                self.off
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left: usize = self.index.into();
        (left, Some(left))
    }
}

impl Buffer2d for GraphicIter5by5 {
    type Item = <Self as Iterator>::Item;

    fn item_at(&self, x: usize, y: usize) -> Option<Self::Item> {
        self.cell_index(x, y).map(|i| {
            let on = (self.bitmap >> (Self::SIZE - 1 - i)) & 0x01 > 0;

            if on {
                self.on
            } else {
                self.off
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::*;

    const LETTER_A_3BY5: u16 = 0b010_101_111_101_101;
    const LETTER_M_5BY5: u32 = 0b10001_11011_10101_10001_10001;

    #[test]
    fn size_hint_fresh_mid_and_exhausted_3by5() {
        let mut glyph = GraphicIter3by5::new(LETTER_A_3BY5);

        assert_eq!(glyph.size_hint(), (15, Some(15)));

        for _ in 0..7 {
            glyph.next().unwrap();
        }
        assert_eq!(glyph.size_hint(), (8, Some(8)));

        for _ in 0..8 {
            glyph.next().unwrap();
        }

        assert_eq!(glyph.size_hint(), (0, Some(0)));
        assert!(glyph.next().is_none());
        assert_eq!(glyph.size_hint(), (0, Some(0)));
    }

    #[test]
    fn size_hint_fresh_mid_and_exhausted_5by5() {
        let mut glyph = GraphicIter5by5::new(LETTER_M_5BY5);

        assert_eq!(glyph.size_hint(), (25, Some(25)));

        for _ in 0..10 {
            glyph.next().unwrap();
        }
        assert_eq!(glyph.size_hint(), (15, Some(15)));

        for _ in 0..15 {
            glyph.next().unwrap();
        }

        assert_eq!(glyph.size_hint(), (0, Some(0)));
        assert!(glyph.next().is_none());
        assert_eq!(glyph.size_hint(), (0, Some(0)));
    }

    /// Streams a glyph sequentially and cross-checks every cell against
    /// `item_at` at the corresponding coordinates.
    #[test]
    fn item_at_matches_iteration() {
        let glyph = GraphicIter3by5::with_on_color(LETTER_A_3BY5, Color::DARK_RED);
        let streamed: Vec<Color> = glyph.clone().collect();

        for i in 0..GraphicIter3by5::SIZE {
            let (x, y) = glyph.get_coordinates(i).unwrap();

            assert_eq!(glyph.item_at(x, y), Some(streamed[i]));
        }

        let glyph = GraphicIter5by5::with_on_color(LETTER_M_5BY5, Color::DARK_GREEN);
        let streamed: Vec<Color> = glyph.clone().collect();

        for i in 0..GraphicIter5by5::SIZE {
            let (x, y) = glyph.get_coordinates(i).unwrap();

            assert_eq!(glyph.item_at(x, y), Some(streamed[i]));
        }
    }

    /// The 5x5 `M` glyph renders the documented bitmap.
    #[test]
    fn m_glyph_bitmap() {
        let glyph = GraphicIter5by5::new(LETTER_M_5BY5);

        for (y, row) in [
            [1, 0, 0, 0, 1],
            [1, 1, 0, 1, 1],
            [1, 0, 1, 0, 1],
            [1, 0, 0, 0, 1],
            [1, 0, 0, 0, 1],
        ]
        .into_iter()
        .enumerate()
        {
            for (x, on) in row.into_iter().enumerate() {
                let expected = if on > 0 { Color::WHITE } else { Color::BLACK };

                assert_eq!(glyph.item_at(x, y), Some(expected));
            }
        }
    }
}
