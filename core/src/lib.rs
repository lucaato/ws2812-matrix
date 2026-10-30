#![no_std]
#![forbid(unsafe_code)]

//! 2D-iterator toolkit for driving WS2812 (NeoPixel) LED matrices.
//!
//! Instead of maintaining a framebuffer, every frame is described as a
//! composition of lazily evaluated 2D iterators: glyphs, chains, positioned
//! elements and composites stream their colors row by row and are rendered
//! once per animation step. The same iterators are consumed
//! identically by the ESP32 firmware ([`ws2812-matrix`](https://github.com/lucaato/ws2812-matrix))
//! and by the desktop emulator, so a frame can be validated without hardware.
//!
//! # Overview
//! - [`Iterator2d`] describes the extents of a 2D pattern, [`Buffer2d`] adds
//!   random access to it.
//! - [`GraphicIter3by5`] and [`GraphicIter5by5`] render bitmap glyphs.
//! - [`Position`], [`Chain2dIter`], [`Composite2dIter`], [`Flip2dIter`] and
//!   [`BufferedIterator2d`] compose, place, join and materialize patterns.
//! - [`scene::team_matrix`] builds the demo animation shared by the firmware
//!   and the emulator.
//! - [`WS2812_T0H_NS`] and friends hold the datasheet timing constants;
//!   [`ns_to_ticks`] converts them to peripheral clock ticks.

extern crate alloc;

mod color;
mod composition;
mod glyph;
mod scene;
mod traits;
mod ws2812;

pub use color::Color;
pub use composition::{
    BufferedIterator2d, Chain2dIter, Composite2dIter, Flip2dIter, HeightMismatch, Position,
};
pub use glyph::{GraphicIter3by5, GraphicIter5by5};
pub use scene::{team_matrix, TeamMatrix};
pub use traits::{Buffer2d, Iterator2d, LineIter2d, PositionedIterator2d};
pub use ws2812::{
    ns_to_ticks, WS2812_RES_NS, WS2812_T0H_NS, WS2812_T0L_NS, WS2812_T1H_NS, WS2812_T1L_NS,
};
