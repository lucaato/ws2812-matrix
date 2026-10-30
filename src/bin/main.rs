#![no_std]
#![no_main]
#![deny(clippy::mem_forget)]

use core::iter;

use alloc::vec::Vec;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::Level;
use esp_hal::main;
use esp_hal::rmt::{PulseCode, Rmt, TxChannel, TxChannelConfig, TxChannelCreator};
use esp_hal::time::{Instant, Rate};
use esp_hal::timer::timg::TimerGroup;
use log::info;

extern crate alloc;

use led_matrix_control::{
    Chain2dIter, Color, Composite2dIter, Flip2dIter, GraphicIter3by5, GraphicIter5by5, Iterator2d,
    LineIter2d, Position, WS2812_RES, WS2812_T0H, WS2812_T0L, WS2812_T1H, WS2812_T1L,
};

#[main]
fn main() -> ! {
    esp_println::logger::init_logger_from_env();

    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let _init = esp_wifi::init(
        timg0.timer0,
        esp_hal::rng::Rng::new(peripherals.RNG),
        peripherals.RADIO_CLK,
    )
    .unwrap();

    let freq = Rate::from_mhz(80);
    let freq_hertz: u64 = freq.as_hz().into();
    let pico_resolution = 1_000_000_000_000 / freq_hertz;

    let duration_t0_high_pico: u64 = Into::<u64>::into(WS2812_T0H.subsec_nanos()) * 1000;
    let duration_t0_high_80mhz: u64 = duration_t0_high_pico / pico_resolution;
    let duration_t0_low_pico: u64 = Into::<u64>::into(WS2812_T0L.subsec_nanos()) * 1000;
    let duration_t0_low_80mhz: u64 = duration_t0_low_pico / pico_resolution;

    let duration_t1_high_pico: u64 = Into::<u64>::into(WS2812_T1H.subsec_nanos()) * 1000;
    let duration_t1_high_80mhz: u64 = duration_t1_high_pico / pico_resolution;
    let duration_t1_low_pico: u64 = Into::<u64>::into(WS2812_T1L.subsec_nanos()) * 1000;
    let duration_t1_low_80mhz: u64 = duration_t1_low_pico / pico_resolution;

    let duration_res_low_pico: u64 = Into::<u64>::into(WS2812_RES.subsec_nanos()) * 1000;
    let duration_res_low_80mhz: u64 = duration_res_low_pico / pico_resolution;

    let channel_config = TxChannelConfig::default()
        .with_clk_divider(1)
        .with_idle_output(true)
        .with_idle_output_level(Level::Low)
        .with_carrier_modulation(false)
        .with_carrier_high(0)
        .with_carrier_low(0)
        .with_carrier_level(Level::Low);
    let rmt = Rmt::new(peripherals.RMT, freq).expect("Expected rmt");
    let mut channel = rmt
        .channel0
        .configure(peripherals.GPIO0, channel_config)
        .expect("expected channel");

    let delay = Delay::new();

    let pulse_code_t0: u32 = PulseCode::new(
        Level::High,
        duration_t0_high_80mhz.try_into().unwrap(),
        Level::Low,
        duration_t0_low_80mhz.try_into().unwrap(),
    );
    let pulse_code_t1: u32 = PulseCode::new(
        Level::High,
        duration_t1_high_80mhz.try_into().unwrap(),
        Level::Low,
        duration_t1_low_80mhz.try_into().unwrap(),
    );
    let pulse_code_res: u32 = PulseCode::new(
        Level::Low,
        duration_res_low_80mhz.try_into().unwrap(),
        Level::High,
        0,
    );

    let _red_lp_repeat: Vec<u32> = iter::repeat_n(pulse_code_t0, 13)
        .chain(iter::repeat_n(pulse_code_t1, 3))
        .chain(iter::repeat_n(pulse_code_t0, 8))
        .chain(iter::once(PulseCode::empty()))
        .collect();

    let iter_letter_t = 0b111_010_010_010_010u16;
    let iter_letter_e = 0b111_100_110_100_111u16;
    let iter_letter_a = 0b010_101_111_101_101u16;
    let iter_letter_m = 0b10001_11011_10101_10001_10001u32;

    let chained = Chain2dIter::new([
        GraphicIter3by5::with_on_color(iter_letter_t, Color::DARK_GREEN),
        GraphicIter3by5::with_on_color(iter_letter_e, Color::DARK_BLUE),
        GraphicIter3by5::with_on_color(iter_letter_a, Color::DARK_GREEN),
    ])
    .unwrap();

    let mut mat_iter = Composite2dIter::new(
        16,
        16,
        Position::new(chained, 1, 1),
        Position::new(
            GraphicIter5by5::with_on_color(iter_letter_m, Color::DARK_BLUE),
            10,
            1,
        ),
        Color::BLACK,
    );

    info!("Starting LED matrix display");

    delay.delay_millis(500);

    loop {
        channel = channel
            .transmit(&[pulse_code_res, PulseCode::empty()])
            .expect("can't transmit RES")
            .wait()
            .expect("wait for RES transmission failed");

        mat_iter.a_iter.y += 1;
        mat_iter.b_iter.y += 1;
        let matrix = Flip2dIter::new(&mat_iter, true).clone();
        for color in matrix {
            let pulse = color.into_pulse_codes(pulse_code_t1, pulse_code_t0);

            channel = channel
                .transmit(&pulse)
                .expect("transmission failed")
                .wait()
                .expect("wait transmission failed");
        }

        channel = channel
            .transmit(&[pulse_code_res, PulseCode::empty()])
            .expect("can't transmit RES")
            .wait()
            .expect("wait for RES transmission failed");

        delay.delay_millis(1000);
    }
}
