/// An RGB color as sent to the WS2812 protocol.
#[derive(Default, Copy, Clone, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const WHITE: Self = Self::new(255, 255, 255);
    pub const DARK_WHITE: Self = Self::new(10, 10, 10);
    pub const DARK_RED: Self = Self::new(10, 0, 0);
    pub const DARK_GREEN: Self = Self::new(0, 10, 0);
    pub const DARK_BLUE: Self = Self::new(0, 0, 10);
    pub const BLACK: Self = Self::new(0, 0, 0);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Returns the 24 data bits for this color, in the order the WS2812
    /// protocol expects them: 8 green bits, then 8 red bits, then 8 blue
    /// bits, each byte sent most-significant bit (MSB) first.
    ///
    /// Index 0 is the first bit to transmit.
    #[must_use]
    pub fn bits(&self) -> [bool; 24] {
        let channels = [self.g, self.r, self.b];
        let mut bits = [false; 24];

        for (i, bit) in bits.iter_mut().enumerate() {
            *bit = (channels[i / 8] << (i % 8)) & 0x80 > 0;
        }

        bits
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_is_all_zeros() {
        assert!(!Color::BLACK.bits().iter().any(|bit| *bit));
    }

    #[test]
    fn white_is_all_ones() {
        assert!(Color::WHITE.bits().iter().all(|bit| *bit));
    }

    #[test]
    fn bits_are_grb_msb_first() {
        // Red: green byte 0, red byte 0b00001010, blue byte 0.
        let bits = Color::new(10, 0, 0).bits();

        let expected = [
            // G
            0, 0, 0, 0, 0, 0, 0, 0, // R = 0b00001010
            0, 0, 0, 0, 1, 0, 1, 0, // B
            0, 0, 0, 0, 0, 0, 0, 0,
        ];

        assert_eq!(bits, expected.map(|c| c > 0));
    }
}
