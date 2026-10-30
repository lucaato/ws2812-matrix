#![no_std]
#![no_main]
#![deny(clippy::mem_forget)]

use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::Level;
use esp_hal::main;
use esp_hal::rmt::{PulseCode, Rmt, TxChannel, TxChannelConfig, TxChannelCreator};
use esp_hal::time::Rate;
use log::info;

use ws2812_matrix_core::{ns_to_ticks, WS2812_T0H_NS, WS2812_T0L_NS, WS2812_T1H_NS, WS2812_T1L_NS};
use ws2812_matrix_core::{team_matrix, Color, Flip2dIter, WS2812_RES_NS};

/// Clock frequency of the RMT peripheral, in hertz.
const RMT_CLOCK_HZ: u32 = 80_000_000;

/// One animation step every second, matching the emulator's default.
const FRAME_DELAY_MS: u32 = 1000;

fn pulse_code(high_ticks: u16, low_ticks: u16) -> u32 {
    PulseCode::new(Level::High, high_ticks, Level::Low, low_ticks)
}

fn encode_color(color: Color, bit_0: u32, bit_1: u32) -> [u32; 25] {
    let mut codes = [0u32; 25];

    for (i, bit) in color.bits().into_iter().enumerate() {
        codes[i] = if bit { bit_1 } else { bit_0 };
    }

    codes
}

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 64 * 1024);

    let t0h = ns_to_ticks(WS2812_T0H_NS, RMT_CLOCK_HZ);
    let t1h = ns_to_ticks(WS2812_T1H_NS, RMT_CLOCK_HZ);

    let bit_0 = pulse_code(t0h, ns_to_ticks(WS2812_T0L_NS, RMT_CLOCK_HZ));
    let bit_1 = pulse_code(t1h, ns_to_ticks(WS2812_T1L_NS, RMT_CLOCK_HZ));
    let reset = pulse_code(ns_to_ticks(WS2812_RES_NS, RMT_CLOCK_HZ), 0);

    let channel_config = TxChannelConfig::default()
        .with_clk_divider(1)
        .with_idle_output(true)
        .with_idle_output_level(Level::Low)
        .with_carrier_modulation(false)
        .with_carrier_high(0)
        .with_carrier_low(0)
        .with_carrier_level(Level::Low);
    let rmt = Rmt::new(peripherals.RMT, Rate::from_mhz(80)).expect("expected rmt");
    let mut channel = rmt
        .channel0
        .configure(peripherals.GPIO0, channel_config)
        .expect("expected channel");

    let delay = Delay::new();

    info!("Starting LED matrix display");

    delay.delay_millis(500);

    let mut t: usize = 0;

    loop {
        channel = channel
            .transmit(&[reset, PulseCode::empty()])
            .expect("can't transmit RES")
            .wait()
            .expect("wait failed");

        let matrix = team_matrix(t);
        let serpentine = Flip2dIter::new(&matrix, true);

        for color in serpentine {
            let codes = encode_color(color, bit_0, bit_1);

            channel = channel
                .transmit(&codes)
                .expect("transmission failed")
                .wait()
                .expect("wait for transmission failed");
        }

        channel = channel
            .transmit(&[reset, PulseCode::empty()])
            .expect("can't transmit RES")
            .wait()
            .expect("wait failed");

        t += 1;
        delay.delay_millis(FRAME_DELAY_MS);
    }
}
