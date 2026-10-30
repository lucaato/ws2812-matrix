/// WS2812 datasheet timings, in nanoseconds:
/// - `T0H`: high time for a logical 0,
/// - `T0L`: low time for a logical 0,
/// - `T1H`: high time for a logical 1,
/// - `T1L`: low time for a logical 1,
/// - `RES`: minimum low time to reset the chain and latch a frame.
pub const WS2812_T0H_NS: u32 = 400;
pub const WS2812_T0L_NS: u32 = 850;
pub const WS2812_T1H_NS: u32 = 800;
pub const WS2812_T1L_NS: u32 = 450;
pub const WS2812_RES_NS: u32 = 50_000;

/// Converts a duration in nanoseconds to ticks of a peripheral running at
/// `clock_hz`, rounded down.
///
/// For the ESP32 RMT clocked at 80 MHz one tick is 12.5 ns, so the
/// datasheet values above map to `T0H` = 32, `T0L` = 68, `T1H` = 64,
/// `T1L` = 36 ticks.
#[must_use]
pub const fn ns_to_ticks(ns: u32, clock_hz: u32) -> u16 {
    ((ns as u64 * clock_hz as u64) / 1_000_000_000) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    const RMT_80MHZ: u32 = 80_000_000;

    #[test]
    fn datasheet_values_in_rmt_ticks() {
        assert_eq!(ns_to_ticks(WS2812_T0H_NS, RMT_80MHZ), 32);
        assert_eq!(ns_to_ticks(WS2812_T0L_NS, RMT_80MHZ), 68);
        assert_eq!(ns_to_ticks(WS2812_T1H_NS, RMT_80MHZ), 64);
        assert_eq!(ns_to_ticks(WS2812_T1L_NS, RMT_80MHZ), 36);
        assert_eq!(ns_to_ticks(WS2812_RES_NS, RMT_80MHZ), 4000);
    }
}
