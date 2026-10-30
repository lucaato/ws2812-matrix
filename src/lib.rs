#![no_std]

extern crate alloc;

use core::array;
use core::iter::{self};
use core::ops::Range;

use alloc::boxed::Box;
use esp_hal::rmt::PulseCode;
use esp_hal::time::Duration;

#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub const WHITE: Color = Color::new(255, 255, 255);
    pub const DARK_WHITE: Color = Color::new(10, 10, 10);
    pub const DARK_RED: Color = Color::new(10, 0, 0);
    pub const DARK_GREEN: Color = Color::new(0, 10, 0);
    pub const DARK_BLUE: Color = Color::new(0, 0, 10);
    pub const BLACK: Color = Color::new(0, 0, 0);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn into_pulse_codes(&self, bit_1: u32, bit_0: u32) -> [u32; 25] {
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

pub trait Iterator2d {
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

pub trait PositionedIterator2d: Iterator2d {
    fn get_x(&self) -> usize;
    fn get_y(&self) -> usize;
}

pub trait Buffer2d: Iterator2d {
    type Item;

    fn into_index(&self, x: usize, y: usize) -> Option<usize> {
        (x < self.get_width() && y < self.get_height()).then(|| y * self.get_width() + x)
    }

    fn item_at(&self, x: usize, y: usize) -> Option<Self::Item>;
}

#[derive(Clone, Debug)]
pub struct GraphicIter3by5 {
    number: u16,
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
pub struct GraphicIter5by5 {
    number: u32,
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
        let left: usize = self.index.into();
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
pub struct Position<I> {
    pub inner: I,
    pub x: usize,
    pub y: usize,
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
}

impl<I> Position<I> {
    pub fn new(inner: I, x: usize, y: usize) -> Self {
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
pub struct Composite2dIter<A, B, E> {
    pub container_width: usize,
    pub container_height: usize,
    pub a_iter: A,
    pub b_iter: B,
    index: usize,
    line_a_intersecion: Option<Range<usize>>,
    line_b_intersecion: Option<Range<usize>>,
    default: E,
}

impl<A, B, E> Composite2dIter<A, B, E>
where
    E: Clone,
{
    pub fn new(
        container_width: usize,
        container_height: usize,
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
    pub fn intersect_line<C>(&self, y: usize, iter_box: &C) -> Option<Range<usize>>
    where
        C: PositionedIterator2d,
    {
        (y >= iter_box.get_y()
            && y < iter_box.get_y() + iter_box.get_height()
            && iter_box.get_x() < self.container_width)
            .then(|| {
                let end = self
                    .container_width
                    .min(iter_box.get_x() + iter_box.get_width());

                iter_box.get_x()..end
            })
    }

    pub fn check_intersection_at(
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

#[derive(Clone, Debug)]
pub struct BufferedIterator2d<T, E> {
    inner: T,
    buffer: Box<[E]>,
}

impl<T, E> BufferedIterator2d<T, E>
where
    T: Iterator2d,
    E: Clone,
{
    pub fn new(inner: T, default: E) -> Self {
        let buffer = iter::repeat_n(default, inner.get_width()).collect();

        Self { inner, buffer }
    }
}

pub trait LineIter2d {
    type Item;
    type Iter: Iterator<Item = Self::Item>;

    fn get_line_iter(self, line: usize) -> Self::Iter;
}

#[derive(Clone, Debug)]
pub struct Composite2dIterLine<'a, A, B, E> {
    pub parent: &'a Composite2dIter<A, B, E>,
    pub a_intersect: Option<Range<usize>>,
    pub b_intersect: Option<Range<usize>>,
    pub start_cell: usize,
    pub end_cell: usize,
    pub y: usize,
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
pub struct Flip2dIter<I, IT> {
    pub inner: I,
    line_iter: IT,
    current_line: usize,
    flip: bool,
}

impl<I, IT> Flip2dIter<I, IT>
where
    I: LineIter2d<Iter = IT> + Copy,
{
    pub fn new(inner: I, flip: bool) -> Self {
        let line_iter = inner.get_line_iter(0);

        Self {
            inner,
            line_iter,
            current_line: 0,
            flip,
        }
    }
}

impl<T> Iterator2d for &T
where
    T: Iterator2d,
{
    fn get_width(&self) -> usize {
        T::get_width(self)
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
                self.flip = !self.flip;
                self.line_iter = self.inner.get_line_iter(self.current_line);
                self.line_iter.next()
            })
        } else {
            self.line_iter.next().or_else(|| {
                if self.current_line >= self.inner.get_height() {
                    return None;
                }

                self.current_line += 1;
                self.flip = !self.flip;
                self.line_iter = self.inner.get_line_iter(self.current_line);
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
pub struct Chain2dIter<const N: usize, T> {
    pub inner: [T; N],
    tot_width: usize,
    counter: usize,
}

impl<const N: usize, T> Chain2dIter<N, T> {
    pub fn new(array: [T; N]) -> Result<Self, ()>
    where
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
                e_range = e_range.end..(e_range.end + e.get_width());

                if e_range.contains(&x) {
                    return e.next();
                }
            }

            None
        })
    }
}

/// WS2812 timing constants (nanoseconds)
pub const WS2812_T0H: Duration = Duration::from_nanos(400);
pub const WS2812_T0L: Duration = Duration::from_nanos(850);
pub const WS2812_T1H: Duration = Duration::from_nanos(800);
pub const WS2812_T1L: Duration = Duration::from_nanos(450);
pub const WS2812_RES: Duration = Duration::from_micros(50);
