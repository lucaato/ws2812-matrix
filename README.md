# ws2812-matrix

[![CI](https://github.com/lucaato/ws2812-matrix/actions/workflows/ci.yml/badge.svg)](https://github.com/lucaato/ws2812-matrix/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

A `no_std` Rust firmware for a WS2812B (NeoPixel) LED matrix on an ESP32-C3, built around a small 2D-iterator library — plus a desktop emulator that runs the **exact same scene code**.

The demo scrolls the letters `T`, `E`, `A`, `M` across a 16x16 matrix. On hardware, the frame is bit-banged through the ESP-HAL **RMT peripheral** with datasheet-accurate WS2812 timings; on the desktop, the identical iterators are read cell by cell into an egui window.

## Why iterators, and not a framebuffer?

Instead of holding a `[[Color; 16]; 16]` and poking pixels, every frame is a *composition of lazily evaluated 2D iterators*:

- glyphs are `GraphicIter3by5`/`GraphicIter5by5` streams;
- `Position` places a pattern, `Chain2dIter` glues patterns side by side, `Composite2dIter` overlays them on a canvas;
- `Flip2dIter` re-streams any line-iterate canvas in serpentine order, matching how WS2812 strips are physically wired.

Patterns are transformed and traversed exactly once per frame, with no bounds-checked intermediate buffers, and the core is `no_std`-clean: the firmware and the desktop emulator share one rendering path, so what you see in the emulator is what lands on the LEDs.

## Repository layout

| Crate | Description |
|---|---|
| [`core`](core/) | `no_std` toolkit: colors, 2D-iterator combinators, bitmap glyphs, WS2812 timing and the shared demo scene |
| [`firmware`](firmware/) | ESP32-C3 binary: RMT channel configuration, WS2812 pulse encoding, animation loop |
| [`emulator`](emulator/) | Desktop viewer (egui/eframe) running the shared scene code |

## Getting started

The provided Nix dev shell is the easiest route (a [direnv](https://direnv.net/) `.envrc` is included):

```bash
nix develop
```

Without Nix you need a stable Rust toolchain (see `rust-toolchain.toml`, including the `riscv32imc-unknown-none-elf` target and `rust-src`) and [espflash](https://github.com/esp-rs/espflash):

```bash
rustup component add rust-src
rustup target add riscv32imc-unknown-none-elf
cargo install espflash
```

### Desktop emulator

From the repository root:

```bash
cargo run -p ws2812-matrix-emulator --release
```

The window shows the 16x16 matrix running the same `TEA M` animation as the firmware. Space toggles play/pause; sliders adjust the animation step time, the zoom level and the display gain (the demo uses low duty-cycle values, so a gain is applied to make them visible against the raw protocol values).

### Firmware

Everything below runs from the `firmware/` directory (its `.cargo/config.toml` selects the RISC-V target):

```bash
cd firmware

# build for riscv32imc-unknown-none-elf
cargo build --release

# build, flash and open the serial monitor (uses the espflash runner)
cargo run --release

# or flash explicitly
espflash flash --chip esp32c3 --monitor target/riscv32imc-unknown-none-elf/release/ws2812-matrix
```

### Tests

The core library is host-testable (`cargo test` runs on the host even though the crate is `no_std`):

```bash
cargo test -p ws2812-matrix-core
```

## Hardware

- Board: ESP32-C3 (tested on Seeed XIAO ESP32-C3 and ESP32-C3-DevKitM-1)
- Matrix: WS2812B addressable LEDs wired as a 16x16 serpentine grid
- Wiring: `GPIO0` -> matrix `DIN`
- Power: the matrix and the control signal need a common ground; consider a level shifter on `DIN` since a 3.3 V data signal can be marginal at 5 V supply
- Serpentine wiring (left-to-right, then right-to-left rows) is expected; `Flip2dIter` re-streams the canvas accordingly

## WS2812 timing

WS2812 LEDs are driven by precise high/low pulse widths per bit, sent green-red-blue, MSB first. The constants live in [`core/src/ws2812.rs`](core/src/ws2812.rs); the firmware converts them to RMT ticks at the 80 MHz clock (12.5 ns resolution) and encodes every bit as an RMT `PulseCode`:

| Param | Value |
|---|---|
| `T0H` | 400 ns |
| `T0L` | 850 ns |
| `T1H` | 800 ns |
| `T1L` | 450 ns |
| `RES` | 50 us (latch/reset) |

A `RES` pulse train is transmitted before and after each frame to latch it.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option, as is common in the embedded Rust ecosystem.
