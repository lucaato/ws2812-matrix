use std::time::Instant;

use eframe::egui;
use ws2812_matrix_core::team_matrix;
use ws2812_matrix_core::Color;

const MATRIX_WIDTH: usize = 16;
const MATRIX_HEIGHT: usize = 16;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([640.0, 680.0])
            .with_title("ws2812-matrix emulator"),
        ..Default::default()
    };

    eframe::run_native(
        "ws2812-matrix emulator",
        options,
        Box::new(|_cc| Ok(Box::new(EmulatorApp::new()))),
    )
}

struct EmulatorApp {
    t: u64,
    playing: bool,
    frame_ms: f64,
    cell_size: f32,
    gain: f32,
    last_frame: Instant,
}

impl EmulatorApp {
    fn new() -> Self {
        Self {
            t: 0,
            playing: true,
            frame_ms: 1000.0 / 4.0,
            cell_size: 32.0,
            gain: 16.0,
            last_frame: Instant::now(),
        }
    }

    /// Reads one full frame of the shared demo scene, row-major.
    fn read_frame(&self) -> [Color; MATRIX_WIDTH * MATRIX_HEIGHT] {
        let mut frame = [Color::BLACK; MATRIX_WIDTH * MATRIX_HEIGHT];

        for (cell, color) in frame.iter_mut().zip(team_matrix(self.t as usize)) {
            *cell = color;
        }

        frame
    }
}

impl eframe::App for EmulatorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::Space)) {
            self.playing = !self.playing;
        }

        let dt = self.last_frame.elapsed().as_secs_f64();
        self.last_frame = Instant::now();

        if self.playing {
            self.t += (dt / (self.frame_ms / 1000.0)) as u64;
        }

        egui::TopBottomPanel::top("controls").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.toggle_value(&mut self.playing, "Play/Pause (space)");

                ui.add(
                    egui::Slider::new(&mut self.frame_ms, 40.0..=2000.0)
                        .logarithmic(true)
                        .text("step time (ms)"),
                );
                ui.add(egui::Slider::new(&mut self.cell_size, 8.0..=40.0).text("zoom"));
                ui.add(
                    egui::Slider::new(&mut self.gain, 1.0..=40.0)
                        .logarithmic(true)
                        .text("display gain"),
                );
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let side = (self.cell_size * MATRIX_WIDTH as f32) + 8.0;
            let response = ui.allocate_response(egui::vec2(side, side), egui::Sense::hover());
            let rect = response.rect;
            let painter = ui.painter_at(rect);

            painter.rect_filled(rect, 4.0, egui::Color32::BLACK);

            let frame = self.read_frame();
            let margin = 4.0;

            for (i, color) in frame.iter().enumerate() {
                let x = (i % MATRIX_WIDTH) as f32;
                let y = (i / MATRIX_WIDTH) as f32;

                let center = rect.min
                    + egui::vec2(margin, margin)
                    + egui::vec2(x * self.cell_size, y * self.cell_size)
                    + egui::vec2(self.cell_size, self.cell_size) * 0.5;

                let radius = self.cell_size * 0.42;

                let brightened = |v: u8| ((v as f32 * self.gain).round().min(255.0)) as u8;

                painter.circle_filled(
                    center,
                    radius,
                    egui::Color32::from_rgb(
                        brightened(color.r),
                        brightened(color.g),
                        brightened(color.b),
                    ),
                );
            }
        });
    }
}
