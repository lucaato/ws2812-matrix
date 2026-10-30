# led_matrix_control

A no_std Rust firmware for driving a WS2812B (NeoPixel) LED matrix using an ESP32-C3 and the ESP-HAL RMT peripheral.

## Hardware
- Board: ESP32-C3 (e.g. Seeed XIAO ESP32C3, ESP32-C3-DevKitM-1)
- LED Matrix: WS2812B addressable LEDs (arranged in a 16x16 grid in this demo)
- Data pin: GPIO0

## Features
- 2D iterator framework for composing and transforming LED patterns
- Combinators: `Composite2dIter`, `Chain2dIter`, `Flip2dIter`, `Position`, `BufferedIterator2d`
- Glyph renderers: `GraphicIter3by5` and `GraphicIter5by5`
- WS2812 timing constants and RMT pulse generation

## Dependencies
Key crates (see `Cargo.toml`):
- `esp-hal` 1.0.0-beta.0 (ESP32-C3, RMT, timers)
- `esp-backtrace`, `esp-println`, `esp-alloc`, `esp-wifi`
- `log`, `heapless`

## Building & Flashing

This project uses `esp-hal` and is configured for `no_std` on ESP32-C3. Recommended tools:
- [`cargo-espflash`](https://github.com/esp-rs/espflash)
- [`espflash`](https://github.com/esp-rs/espflash) CLI
- [`probe-rs`](https://probe.rs/) (alternative)

### Prerequisites
Install the Rust toolchain for ESP (this repo has `rust-toolchain.toml`). Also install `espflash`:
```bash
cargo install cargo-espflash espflash
```

### Build
```bash
cargo build --release
```

### Flash
Connect the ESP32-C3 via USB and flash:
```bash
cargo espflash flash --release --monitor
# or
espflash flash target/riscv32imc-unknown-none-elf/release/led_matrix_control --monitor
```

## WS2812 RMT Timing
WS2812 (GRB) is driven by precise high/low pulse widths. This project uses the ESP32 RMT peripheral at 80 MHz (12.5 ns resolution). Timing constants are defined in `src/lib.rs`:
- T0H: 400 ns (logical 0 high)
- T0L: 850 ns (logical 0 low)
- T1H: 800 ns (logical 1 high)
- T1L: 450 ns (logical 1 low)
- RES: 50 μs (reset/latch)

The code converts these to RMT clock ticks and encodes each bit via `PulseCode`. A reset sequence `[RES, empty]` is transmitted to latch the frame.

## Project Structure
- `src/lib.rs` - Reusable components (color, 2D iterators/combinators, glyphs, WS2812 timing)
- `src/bin/main.rs` - Application entrypoint: RMT setup, demo pattern animation on GPIO0
