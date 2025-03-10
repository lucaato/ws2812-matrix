#![no_std]
#![no_main]
#![deny(clippy::mem_forget)]

use core::array;
use core::iter::{self};
use core::ops::Range;
use core::time::Duration;

use alloc::boxed::Box;
use alloc::vec::Vec;
use esp_backtrace as _;
use esp_hal::clock::CpuClock;
use esp_hal::delay::Delay;
use esp_hal::gpio::Level;
use esp_hal::main;
use esp_hal::rmt::{PulseCode, Rmt, TxChannel, TxChannelConfig, TxChannelCreator};
use esp_hal::time::{Instant, Rate};
use esp_hal::timer::timg::TimerGroup;
use esp_println::{print, println};
use log::info;

extern crate alloc;

/// T0H duration time (0 code, high voltage time)
const WS2812_T0H: Duration = Duration::from_nanos(400);
/// T0L duration time (0 code, low voltage time)
const WS2812_T0L: Duration = Duration::from_nanos(850);
/// T1H duration time (1 code, high voltage time)
const WS2812_T1H: Duration = Duration::from_nanos(800);
/// T1L duration time (1 code, low voltage time)
const WS2812_T1L: Duration = Duration::from_nanos(450);
/// RES duration time (RES code, low voltage time)
const WS2812_RES: Duration = Duration::from_micros(50);

// pub fn new_with_duration(
//     ticks_hz: Hertz,
//     pin_state: PinState,
//     duration: &Duration,
// ) -> Result<Self, EspError> {
//     let ticks = PulseTicks::new_with_duration(ticks_hz, duration)?;
//     Ok(Self::new(pin_state, ticks))
// }

#[derive(Default, Clone, Debug, PartialEq, Eq)]
struct Color {
    r: u8,
    g: u8,
    b: u8,
}

impl Color {
    pub const WHITE: Color = Color::new(255, 255, 255);
    pub const DARK_WHITE: Color = Color::new(10, 10, 10);
    pub const DARK_RED: Color = Color::new(10, 0, 0);
    pub const DARK_GREEN: Color = Color::new(0, 10, 0);
    pub const DARK_BLUE: Color = Color::new(0, 0, 10);
    pub const BLACK: Color = Color::new(0, 0, 0);

    const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    fn into_pulse_codes(&self, bit_1: u32, bit_0: u32) -> [u32; 25] {
        let order = [self.g, self.r, self.b];
        array::from_fn(|i| {
            if i >= 24 {
                0
            } else if (order[i / 8] << i % 8) & 0x40 > 0 {
                bit_1
            } else {
                bit_0
            }
        })
    }
}

trait Iterator2d {
    fn get_width(&self) -> usize;
    fn get_height(&self) -> usize;

    #[inline]
    fn get_coordinates(&self, i: usize) -> Option<(usize, usize)> {
        let y = i / self.get_width();

        (y < self.get_height()).then(|| {
            let x = i % self.get_width();

            (x, y)
        })
    }
}

trait PositionedIterator2d: Iterator2d {
    fn get_x(&self) -> usize;
    fn get_y(&self) -> usize;
}

trait Buffer2d: Iterator2d {
    type Item;

    fn into_index(&self, x: usize, y: usize) -> Option<usize> {
        (x < self.get_width() && y < self.get_height()).then(|| y * self.get_width() + x)
    }

    fn item_at(&self, x: usize, y: usize) -> Option<Self::Item>;
}

#[derive(Clone, Debug)]
struct GraphicIter3by5 {
    // size of the number is 5x3 = 15 last bit is unused
    number: u16,
    index: u8,
    on: Color,
    off: Color,
}

impl GraphicIter3by5 {
    const WIDTH: usize = 3;
    const HEIGHT: usize = 5;
    const SIZE: usize = (Self::WIDTH * Self::HEIGHT);

    fn new(bit_list: u16) -> Self {
        Self::with_on_color(bit_list, Color::DARK_WHITE)
    }

    fn with_on_color(bit_list: u16, on: Color) -> Self {
        Self {
            number: bit_list,
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

            if (self.number >> i) & 0x01 > 0 {
                self.on.clone()
            } else {
                self.off.clone()
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // index is initialized as 16
        let left: usize = self.index.into();
        (left, Some(left))
    }
}

impl Buffer2d for GraphicIter3by5 {
    type Item = <Self as Iterator>::Item;

    fn item_at(&self, x: usize, y: usize) -> Option<Self::Item> {
        self.into_index(x, y).map(|i| {
            let current = (self.number >> (Self::SIZE - 1 - i)) & 0x01 > 0;

            if current {
                self.on.clone()
            } else {
                self.off.clone()
            }
        })
    }
}
#[derive(Clone, Debug)]
struct GraphicIter5by5 {
    // size of the number is 5x3 = 15 last bit is unused
    number: u32,
    index: u8,
    on: Color,
    off: Color,
}

impl GraphicIter5by5 {
    const WIDTH: usize = 5;
    const HEIGHT: usize = 5;
    const SIZE: usize = (Self::WIDTH * Self::HEIGHT);

    fn new(bit_list: u32) -> Self {
        Self::with_on_color(bit_list, Color::WHITE)
    }

    fn with_on_color(bit_list: u32, on: Color) -> Self {
        Self {
            number: bit_list,
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

            if (self.number >> i) & 0x01 > 0 {
                self.on.clone()
            } else {
                self.off.clone()
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        // index is initialized as 16
        let left: usize = (self.index - 1).into();
        (left, Some(left))
    }
}

impl Buffer2d for GraphicIter5by5 {
    type Item = <Self as Iterator>::Item;

    fn item_at(&self, x: usize, y: usize) -> Option<Self::Item> {
        self.into_index(x, y).map(|i| {
            let current = (self.number >> (Self::SIZE - 1 - i)) & 0x01 > 0;

            if current {
                self.on.clone()
            } else {
                self.off.clone()
            }
        })
    }
}

#[derive(Clone, Debug)]
struct Position<I> {
    inner: I,
    x: usize,
    y: usize,
}

impl<I> Iterator for Position<I>
where
    I: Iterator,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
    // NOTE should redirect all members
}

impl<I> Position<I> {
    fn new(inner: I, x: usize, y: usize) -> Self {
        Self { inner, x, y }
    }
}

impl<I> Iterator2d for Position<I>
where
    I: Iterator2d,
{
    fn get_width(&self) -> usize {
        self.inner.get_width()
    }

    fn get_height(&self) -> usize {
        self.inner.get_height()
    }
}

impl<I> PositionedIterator2d for Position<I>
where
    I: Iterator2d,
{
    fn get_x(&self) -> usize {
        self.x
    }

    fn get_y(&self) -> usize {
        self.y
    }
}

impl<I> Buffer2d for Position<I>
where
    I: Buffer2d,
{
    type Item = I::Item;

    fn item_at(&self, x: usize, y: usize) -> Option<Self::Item> {
        self.inner.item_at(x, y)
    }
}

#[derive(Clone, Debug)]
struct Composite2dIter<A, B, E> {
    container_width: usize,
    container_height: usize,
    a_iter: A,
    b_iter: B,
    index: usize,
    line_a_intersecion: Option<Range<usize>>,
    line_b_intersecion: Option<Range<usize>>,
    default: E,
}

impl<A, B, E> Composite2dIter<A, B, E>
where
    E: Clone,
{
    fn new(
        container_width: usize,
        container_height: usize,
        // a is always on top of b if overlapping
        a_iter: A,
        b_iter: B,
        default: E,
    ) -> Self {
        Self {
            container_width,
            container_height,
            a_iter,
            b_iter,
            line_a_intersecion: None,
            line_b_intersecion: None,
            index: 0,
            default,
        }
    }
}
impl<A, B, E> Iterator2d for Composite2dIter<A, B, E> {
    fn get_width(&self) -> usize {
        self.container_width
    }

    fn get_height(&self) -> usize {
        self.container_height
    }
}

impl<A, B, E> Composite2dIter<A, B, E> {
    fn intersect_line<C>(&self, y: usize, iter_box: &C) -> Option<Range<usize>>
    where
        C: PositionedIterator2d,
    {
        // is the box contained in this line
        (y >= iter_box.get_y()
            && y < iter_box.get_y() + iter_box.get_height()
        // is the box start inside the row ?
            && iter_box.get_x() < self.container_width)
            .then(|| {
                let end = self
                    .container_width
                    .min(iter_box.get_x() + iter_box.get_width());

                iter_box.get_x()..end
            })
    }

    // TODO should i enclose this match logic in an enum ?
    // enum Composite2dIterIntersection {
    //     IntersectionA(),
    //     IntersectionB(),
    //     None
    // }

    fn check_intersection_at(
        &self,
        x: usize,
        y: usize,
        a_intersect: &Option<Range<usize>>,
        b_intersect: &Option<Range<usize>>,
    ) -> Option<E>
    where
        E: Clone,
        A: Buffer2d<Item = E> + PositionedIterator2d,
        B: Buffer2d<Item = E> + PositionedIterator2d,
    {
        // TODO kinda sad we have to subtract a random value that could underflow
        match (&a_intersect, &b_intersect) {
            (None, None) => self.default.clone().into(),
            (None, Some(b)) => {
                if b.contains(&x) {
                    self.b_iter.item_at(x - b.start, y - self.b_iter.get_y())
                } else {
                    self.default.clone().into()
                }
            }
            (Some(a), None) => {
                if a.contains(&x) {
                    self.a_iter.item_at(x - a.start, y - self.a_iter.get_y())
                } else {
                    self.default.clone().into()
                }
            }
            (Some(a), Some(b)) => {
                if a.contains(&x) {
                    self.a_iter.item_at(x - a.start, y - self.a_iter.get_y())
                } else if b.contains(&x) {
                    self.b_iter.item_at(x - b.start, y - self.b_iter.get_y())
                } else {
                    self.default.clone().into()
                }
            }
        }
    }
    // FIXME storing the line in the container means that for nested iterators you have multiple line stored in different containers
    // i should store it only if i have to flip it or even better implement a seekable iterator that lets me seek and reverse the direction
    // so that i don't have to store anything
    // FIXME final version store it only in a FlipIterator so that only one container is used
}

impl<A, B, E> Iterator for Composite2dIter<A, B, E>
where
    E: Clone,
    A: Iterator<Item = E> + PositionedIterator2d,
    B: Iterator<Item = E> + PositionedIterator2d,
{
    type Item = E;

    fn next(&mut self) -> Option<Self::Item> {
        self.get_coordinates(self.index).map(|(x, y)| {
            self.index += 1;

            // first time called for new row
            if x == 0 {
                self.line_a_intersecion = self.intersect_line(y, &self.a_iter);
                self.line_b_intersecion = self.intersect_line(y, &self.b_iter);
            }

            match (&self.line_a_intersecion, &self.line_b_intersecion) {
                (None, None) => self.default.clone(),
                (None, Some(b)) => {
                    if b.contains(&x) {
                        self.b_iter.next().unwrap()
                    } else {
                        self.default.clone()
                    }
                }
                (Some(a), None) => {
                    if a.contains(&x) {
                        self.a_iter.next().unwrap()
                    } else {
                        self.default.clone()
                    }
                }
                (Some(a), Some(b)) => {
                    if a.contains(&x) {
                        self.a_iter.next().unwrap()
                    } else if b.contains(&x) {
                        self.b_iter.next().unwrap()
                    } else {
                        self.default.clone()
                    }
                }
            }
        })
    }
}

// NOTE buffer 2d impelemented for composite iter is expensive since access can't be controlled (are not line by line)
// iter that are "native" Buffer2d that have less checking involved and less memory used
// this buffer uses n (n == Iterator2d::get_width) * s (s = sizeof<E>) cells of memory to store the result and allow iteration
// line by line of a double ended iterator so that the underlying iter (that doesn't implement Buffer2d) can be flipped line by line
struct BufferedIterator2d<T, E> {
    inner: T,
    // line buffer that allows a normal iterator to be treated as a LineIter2d
    // by storing the line in a buffer
    buffer: Box<[E]>,
}

impl<T, E> BufferedIterator2d<T, E>
where
    T: Iterator2d,
    E: Clone,
{
    fn new(inner: T, default: E) -> Self {
        let buffer = iter::repeat_n(default, inner.get_width()).collect();

        Self { inner, buffer }
    }
}

// TODO verify if you can use the same trick as in atomics and locks where you take in the lifetime of a mutable reference
// but use immutable references inside, this will mean that as long as the references are alive we can have unique access to the value
impl<'a, T, E> LineIter2d for &'a mut BufferedIterator2d<T, E>
where
    T: Copy,
    T: LineIter2d<Item = E> + Iterator2d + Iterator<Item = E>,
{
    type Item = E;
    type Iter = BufferedIterator2dLineIter<'a, T, E>;

    fn get_line_iter(self, line: usize) -> Self::Iter {
        let line = self.inner.get_line_iter(line);

        // buffer line so that the iter can fetch new elements from both sides (DoubleEndedIterator)
        for (buffered, e) in self.buffer.iter_mut().zip(line) {
            *buffered = e;
        }

        BufferedIterator2dLineIter::new(self)
    }
}

struct BufferedIterator2dLineIter<'a, T, E> {
    // keep a reference so that we can access the buffer stored in the parent
    parent: &'a BufferedIterator2d<T, E>,
    start_cell: usize,
    end_cell: usize,
}

impl<'a, T, E> BufferedIterator2dLineIter<'a, T, E> {
    fn new(parent: &'a BufferedIterator2d<T, E>) -> Self {
        Self {
            parent,
            start_cell: 0,
            end_cell: parent.buffer.len(),
        }
    }
}

impl<'a, T, E> Iterator for BufferedIterator2dLineIter<'a, T, E> {
    type Item = E;

    fn next(&mut self) -> Option<Self::Item> {
        (self.start_cell < self.end_cell)
            .then_some(self.start_cell)
            .and_then(|x| {
                self.start_cell += 1;

                // self.parent
                //     .check_intersection_at(x, self.y, &self.a_intersect, &self.b_intersect)

                todo!()
            })
    }
}

impl<'a, T, E> DoubleEndedIterator for BufferedIterator2dLineIter<'a, T, E> {
    fn next_back(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

// TODO maybe this should take ownership and implement a method that
// switches from one line to the other but I'm not sure we can also store
// the current position and buffer in the child even for the buffereIterator2dLine
// I'm not sure that recreating the buffer for each line is really a problem here
trait LineIter2d {
    type Item;
    type Iter: Iterator<Item = Self::Item>;

    fn get_line_iter(self, line: usize) -> Self::Iter;
}

impl<'a, A, B, E> LineIter2d for &'a Composite2dIter<A, B, E>
where
    E: Clone,
    A: Buffer2d<Item = E> + PositionedIterator2d,
    B: Buffer2d<Item = E> + PositionedIterator2d,
{
    type Item = E;
    type Iter = Composite2dIterLine<'a, A, B, E>;

    fn get_line_iter(self, line: usize) -> Self::Iter {
        let y = line;

        let a_intersection = self.intersect_line(y, &self.a_iter);
        let b_intersection = self.intersect_line(y, &self.b_iter);

        Self::Iter {
            parent: self,
            a_intersect: a_intersection,
            b_intersect: b_intersection,
            start_cell: 0,
            end_cell: self.container_width,
            y,
        }
    }
}

#[derive(Clone, Debug)]
struct Composite2dIterLine<'a, A, B, E> {
    parent: &'a Composite2dIter<A, B, E>,
    a_intersect: Option<Range<usize>>,
    b_intersect: Option<Range<usize>>,
    start_cell: usize,
    end_cell: usize,
    y: usize,
}

impl<'a, A, B, E> Iterator for Composite2dIterLine<'a, A, B, E>
where
    E: Clone,
    A: Buffer2d<Item = E> + PositionedIterator2d,
    B: Buffer2d<Item = E> + PositionedIterator2d,
{
    type Item = E;

    fn next(&mut self) -> Option<Self::Item> {
        (self.start_cell < self.end_cell)
            .then_some(self.start_cell)
            .and_then(|x| {
                self.start_cell += 1;

                self.parent
                    .check_intersection_at(x, self.y, &self.a_intersect, &self.b_intersect)
            })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.end_cell - self.start_cell;

        (remaining, Some(remaining))
    }
}

impl<'a, A, B, E> DoubleEndedIterator for Composite2dIterLine<'a, A, B, E>
where
    E: Clone,
    A: Buffer2d<Item = E> + PositionedIterator2d,
    B: Buffer2d<Item = E> + PositionedIterator2d,
{
    fn next_back(&mut self) -> Option<Self::Item> {
        (self.end_cell > self.start_cell)
            .then(|| self.end_cell - 1)
            .and_then(|x| {
                self.end_cell = x;

                self.parent
                    .check_intersection_at(x, self.y, &self.a_intersect, &self.b_intersect)
            })
    }
}

#[derive(Clone, Debug)]
struct Flip2dIter<I, IT> {
    inner: I,
    line_iter: IT,
    current_line: usize,
    flip: bool,
}

impl<I, IT> Flip2dIter<I, IT>
where
    I: LineIter2d<Iter = IT> + Copy,
{
    fn new(inner: I, flip: bool) -> Self {
        let line_iter = inner.get_line_iter(0);

        Self {
            inner,
            line_iter,
            current_line: 0,
            flip,
        }
    }
}

// blanket implementation for all Iterator2d references
impl<T> Iterator2d for &T
where
    T: Iterator2d,
{
    fn get_width(&self) -> usize {
        T::get_width(&self)
    }

    fn get_height(&self) -> usize {
        T::get_height(self)
    }
}

impl<I, T> Iterator2d for Flip2dIter<I, T>
where
    I: Iterator2d,
{
    fn get_width(&self) -> usize {
        self.inner.get_width()
    }

    fn get_height(&self) -> usize {
        self.inner.get_height()
    }
}

impl<I, IT> Iterator for Flip2dIter<I, IT>
where
    // require copy since we are passing in a reference
    I: Iterator2d + LineIter2d<Iter = IT> + Copy,
    IT: DoubleEndedIterator,
{
    type Item = <I::Iter as Iterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        if self.flip {
            self.line_iter.next_back().or_else(|| {
                if self.current_line >= self.inner.get_height() {
                    return None;
                }

                self.current_line += 1;
                // every line we invert flip
                self.flip = !self.flip;
                self.line_iter = self.inner.get_line_iter(self.current_line);
                // since we inverted the flip bool the 'next' should be the opposite
                self.line_iter.next()
            })
        } else {
            self.line_iter.next().or_else(|| {
                if self.current_line >= self.inner.get_height() {
                    return None;
                }

                self.current_line += 1;
                // every line we invert flip
                self.flip = !self.flip;
                self.line_iter = self.inner.get_line_iter(self.current_line);
                // since we inverted the flip bool the 'next' should be the opposite
                self.line_iter.next_back()
            })
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining_lines_items =
            (self.inner.get_height() - self.current_line) * self.inner.get_width();

        let current_line_hint = self.line_iter.size_hint();

        (
            current_line_hint.0 + remaining_lines_items,
            current_line_hint
                .1
                .map(|upper| upper + remaining_lines_items),
        )
    }
}

#[derive(Clone, Debug)]
struct Chain2dIter<const N: usize, T> {
    inner: [T; N],
    tot_width: usize,
    counter: usize,
}

impl<const N: usize, T> Chain2dIter<N, T> {
    // TODO fast don't want to deal with errors now :D
    fn new(array: [T; N]) -> Result<Self, ()>
    where
        // TODO NOTE that position will be ignored
        T: Iterator2d,
    {
        let Some(first) = array.first() else {
            return Err(());
        };

        let mut tot_width = first.get_width();

        for window in array.windows(2) {
            let (i, j) = (&window[0], &window[1]);

            if i.get_height() != j.get_height() {
                return Err(());
            }

            tot_width += j.get_width();
        }

        Ok(Self {
            inner: array,
            tot_width,
            counter: 0,
        })
    }
}

impl<const N: usize, T> Buffer2d for Chain2dIter<N, T>
where
    T: Buffer2d,
{
    type Item = <T as Buffer2d>::Item;

    fn item_at(&self, x: usize, y: usize) -> Option<Self::Item> {
        let mut e_range = 0..0;

        for e in self.inner.iter() {
            // cache it in an array if needed
            e_range = e_range.end..(e_range.end + e.get_width());

            if e_range.contains(&x) {
                let x = x - e_range.start;
                return e.item_at(x, y);
            }
        }

        None
    }
}

impl<const N: usize, T> Iterator2d for Chain2dIter<N, T>
where
    T: Iterator2d,
{
    fn get_width(&self) -> usize {
        self.tot_width
    }

    fn get_height(&self) -> usize {
        // new checks that height are all equal and that there's at least one element
        self.inner[0].get_height()
    }
}

impl<const N: usize, T> Iterator for Chain2dIter<N, T>
where
    T: Iterator2d + Iterator,
{
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.get_coordinates(self.counter).and_then(|(x, _y)| {
            self.counter += 1;
            let mut e_range = 0..0;

            for e in self.inner.iter_mut() {
                // cache it in an array if needed
                e_range = e_range.end..(e_range.end + e.get_width());

                if e_range.contains(&x) {
                    return e.next();
                }
            }

            None
        })
    }

    // fn size_hint(&self) -> (usize, Option<usize>) {
    //     ()
    // }
}

#[main]
fn main() -> ! {
    // generator version: 0.3.1

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

    //let mut matrix = gpio::Output::new(peripherals.GPIO0, Level::Low, OutputConfig::default());

    let freq = Rate::from_mhz(80);
    // 1 / 80.000.000 = 0.000 000 012 5 this means that the minimum resolution
    // is 12 nanoseconds
    // so
    let freq_hertz: u64 = freq.as_hz().into();
    // this should be equivalent to (1 / 80Mhz) * 10^12 to have the minimum pico resolution
    // (it would be 12,5 nanos so to avoid loosing that half nano i use picoseconds)
    let pico_resolution = 1_000_000_000_000 / freq_hertz;

    info!("Pico resolution {}picos", pico_resolution);

    let duration_T0_high_pico: u64 = Into::<u64>::into(WS2812_T0H.subsec_nanos()) * 1000;
    let duration_T0_high_80mhzclock: u64 = duration_T0_high_pico / pico_resolution;
    let duration_T0_low_pico: u64 = Into::<u64>::into(WS2812_T0L.subsec_nanos()) * 1000;
    let duration_T0_low_80mhzclock: u64 = duration_T0_low_pico / pico_resolution;

    info!("Durations with a 80 Mhz clock");
    info!(
        "T0 high {} pulses\nT0 low {} pulses",
        duration_T0_high_80mhzclock, duration_T0_low_80mhzclock,
    );

    let duration_T1_high_pico: u64 = Into::<u64>::into(WS2812_T1H.subsec_nanos()) * 1000;
    let duration_T1_high_80mhzclock: u64 = duration_T1_high_pico / pico_resolution;
    let duration_T1_low_pico: u64 = Into::<u64>::into(WS2812_T1L.subsec_nanos()) * 1000;
    let duration_T1_low_80mhzclock: u64 = duration_T1_low_pico / pico_resolution;
    info!(
        "T1 high {} pulses\nT1 low {} pulses",
        duration_T1_high_80mhzclock, duration_T1_low_80mhzclock,
    );

    let duration_RES_low_pico: u64 = Into::<u64>::into(WS2812_RES.subsec_nanos()) * 1000;
    let duration_RES_low_80mhzclock: u64 = duration_RES_low_pico / pico_resolution;
    info!("RES low {} pulses", duration_RES_low_80mhzclock,);

    let channel_config = TxChannelConfig::default()
        .with_clk_divider(1)
        // NOTE the solution to the first green led is to enable idle output to Low so
        // that any non transmission results in a reset of the transmission
        .with_idle_output(true)
        .with_idle_output_level(Level::Low)
        .with_carrier_modulation(false)
        .with_carrier_high(0)
        .with_carrier_low(0)
        .with_carrier_level(Level::Low);
    let rmt = Rmt::new(peripherals.RMT, freq).expect("Expected rmt");
    let mut channel = rmt
        .channel0
        .configure(peripherals.GPIO0, channel_config.clone())
        .expect("expected channel");

    let delay = Delay::new();

    let pulse_code_t0: u32 = PulseCode::new(
        Level::High,
        duration_T0_high_80mhzclock.try_into().unwrap(),
        Level::Low,
        duration_T0_low_80mhzclock.try_into().unwrap(),
    );
    let pulse_code_t1: u32 = PulseCode::new(
        Level::High,
        duration_T1_high_80mhzclock.try_into().unwrap(),
        Level::Low,
        duration_T1_low_80mhzclock.try_into().unwrap(),
    );
    let pulse_code_res: u32 = PulseCode::new(
        Level::Low,
        duration_RES_low_80mhzclock.try_into().unwrap(),
        Level::High,
        0,
    );

    // lp stands for low power only the least significant bit is set
    let green_lp: Vec<u32> = iter::repeat_n(pulse_code_t0, 5)
        .chain(iter::repeat_n(pulse_code_t1, 3))
        .chain(iter::repeat_n(pulse_code_t0, 16))
        // important this stops the loop, without this it transmits endlessly
        .chain(iter::once(PulseCode::empty()))
        .collect();
    let red_lp: Vec<u32> = iter::repeat_n(pulse_code_t0, 13)
        .chain(iter::repeat_n(pulse_code_t1, 3))
        .chain(iter::repeat_n(pulse_code_t0, 8))
        // important this stops the loop, without this it transmits endlessly
        .chain(iter::once(PulseCode::empty()))
        .collect();
    let blue_lp: Vec<u32> = iter::repeat_n(pulse_code_t0, 21)
        .chain(iter::repeat_n(pulse_code_t1, 3))
        // important this stops the loop, without this it transmits endlessly
        .chain(iter::once(PulseCode::empty()))
        .collect();
    let off: Vec<u32> = iter::repeat_n(pulse_code_t0, 24)
        // important this stops the loop, without this it transmits endlessly
        .chain(iter::once(PulseCode::empty()))
        .collect();
    let red_lp_repeat: Vec<u32> = iter::from_fn(|| {
        Some(
            red_lp
                .iter()
                // do not take the empty pulse code
                .take(24)
                .copied(),
        )
    })
    .take(17)
    .flat_map(|i| i)
    // important this stops the loop, without this it transmits endlessly
    .chain(iter::once(PulseCode::empty()))
    .collect();

    // every number has a width of 3 pixel and height of 5
    let iter_num_0 = 0b010_101_101_101_010u16;
    let iter_num_1 = 0b110_010_010_010_010u16;
    let iter_num_2 = 0b110_001_010_100_111u16;
    let iter_num_3 = 0b110_001_110_001_110u16;
    // let iter_num_4 = 0b110_001_010_100_111u16;
    // let iter_num_5 = 0b110_001_010_100_111u16;
    let iter_letter_T = 0b111_010_010_010_010u16;
    let iter_letter_E = 0b111_100_110_100_111u16;
    let iter_letter_A = 0b010_101_111_101_101u16;
    let iter_letter_M = 0b10001_11011_10101_10001_10001u32;

    let chained = Chain2dIter::new([
        GraphicIter3by5::with_on_color(iter_letter_T, Color::DARK_GREEN),
        GraphicIter3by5::with_on_color(iter_letter_E, Color::DARK_BLUE),
        GraphicIter3by5::with_on_color(iter_letter_A, Color::DARK_GREEN),
    ])
    .unwrap();

    let mut mat_iter = Composite2dIter::new(
        16,
        16,
        Position::new(chained, 1, 1),
        Position::new(
            GraphicIter5by5::with_on_color(iter_letter_M, Color::DARK_BLUE),
            10,
            1,
        ),
        Color::BLACK,
    );

    let mat_iter_23 = Composite2dIter::new(
        16,
        16,
        Position::new(
            GraphicIter3by5::with_on_color(iter_num_2, Color::DARK_WHITE),
            0,
            2,
        ),
        Position::new(
            GraphicIter3by5::with_on_color(iter_num_3, Color::DARK_WHITE),
            4,
            2,
        ),
        Color::default(),
    );

    // let mut flipped_iter = Flip2dIter::new(&mat_iter_23, true);

    // info!("mat iter {:?}", flipped_iter);

    info!("printing iter");
    for i in (&mat_iter_23).get_line_iter(1) {
        println!("{i:?}, ");
    }

    // delay.delay_millis(60000);

    // let mut flipped_iter = Flip2dIter::new(&mat_iter, true);
    // let mut flipped_iter = Flip2dIter::new(&mat_iter_23, true);
    // let mut flipped_iter = mat_iter_23;

    let start = Instant::now();
    let matrix_precomputed = Vec::from_iter(Flip2dIter::new(&mat_iter, true).clone());
    info!(
        "Time took to compute the whole iterator: {}",
        start.elapsed().as_micros()
    );
    let start = Instant::now();
    let matrix_precomputed_number = Vec::from_iter(GraphicIter3by5::with_on_color(
        iter_num_0,
        Color::DARK_WHITE,
    ));
    core::hint::black_box(matrix_precomputed_number);
    info!(
        "Time took to compute the whole iterator (just a number): {}",
        start.elapsed().as_micros()
    );

    {
        let mut mat_iter = mat_iter.clone();
        let start = Instant::now();
        for _i in 0..1 {
            let e = mat_iter.next();
            core::hint::black_box(e);
        }
        info!(
            "Time took to compute 1 step of the mat iterator: {}",
            start.elapsed().as_micros()
        );
    }

    {
        let mut mat_iter = mat_iter.clone().skip(16);
        let start = Instant::now();
        for _i in 0..1 {
            let e = mat_iter.next();
            core::hint::black_box(e);
        }
        info!(
            "Time took to compute 1 step of the mat iterator (the one that computes the line): {}",
            start.elapsed().as_micros()
        );
    }

    let start = Instant::now();
    let mut test_iter = 0..256;
    for _i in 0..1 {
        let e = test_iter.next();
        core::hint::black_box(e);
    }
    info!(
        "Time took to compute 1 step of the simple range iterator: {}",
        start.elapsed().as_micros()
    );

    let start = Instant::now();
    let test_iter = Vec::from_iter(0..256);
    core::hint::black_box(test_iter);
    info!(
        "Time took to compute 256 step of the simple range iterator: {}",
        start.elapsed().as_micros()
    );

    info!("waiting before starting the loop");
    delay.delay_millis(1000);

    loop {
        // resetting first
        channel = channel
            // TODO the bug in the library here is probably the condition in which pulse code empty is checked
            .transmit(&[pulse_code_res, PulseCode::empty()])
            .expect("can't transmit RES")
            .wait()
            .expect("wait for RES transmission failed");

        mat_iter.a_iter.y += 1;
        mat_iter.b_iter.y += 1;
        let matrix = Flip2dIter::new(&mat_iter, true).clone();
        for color in matrix {
            let pulse = color.into_pulse_codes(pulse_code_t1, pulse_code_t0);

            //info!("Pulses for pixel: {:?}", pulse);

            channel = channel
                .transmit(&pulse)
                .expect("transmission filed")
                .wait()
                .expect("wait transmission failed");
        }

        // for (i, color) in [&green_lp, &red_lp, &blue_lp, &off].into_iter().enumerate() {
        //     info!("setting color {}", i);

        //     let transaction = channel
        //         .transmit_continuously_with_loopcount(2, color)
        //         .expect("Transmission failed");

        //     while !transaction.is_loopcount_interrupt_set() {
        //         // in theory the unit of transmission is 24 bit composed of 1s and 0s
        //         // each last 1.25 microsecond
        //         // 24 * 1.25 = 30 microseconds for the transmission of 1 color
        //         // this means that we have to delay less than that
        //         // 15 could be totally off if happens right before the end of the transmission
        //         delay.delay_micros(1);
        //     }

        //     channel = transaction.stop().expect("Could not stop the transaction");

        //     //info!("Waiting 2 seconds before changing color");
        //     //delay.delay_millis(2000);
        // }

        channel = channel
            // TODO the bug in the library here is probably the condition in which pulse code empty is checked
            .transmit(&[pulse_code_res, PulseCode::empty()])
            .expect("can't transmit RES")
            .wait()
            .expect("wait for RES transmission failed");

        delay.delay_millis(1000);
    }

    // for inspiration have a look at the examples at https://github.com/esp-rs/esp-hal/tree/esp-hal-v1.0.0-beta.0/examples/srcs BufferedIterator2d<T>
}
