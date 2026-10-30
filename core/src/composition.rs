use alloc::boxed::Box;
use core::iter::repeat_n;
use core::ops::Range;

use crate::traits::{Buffer2d, Iterator2d, LineIter2d, PositionedIterator2d};
use crate::Color;

/// Places a 2D pattern at `(x, y)` inside a bigger canvas.
#[derive(Clone, Debug)]
pub struct Position<I> {
    pub inner: I,
    pub x: usize,
    pub y: usize,
}

impl<I> Position<I> {
    pub fn new(inner: I, x: usize, y: usize) -> Self {
        Self { inner, x, y }
    }
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

/// Overlays two positioned patterns on a `(width, height)` canvas, streaming
/// the full canvas row by row.
///
/// Wherever a pattern covers the cell, its color is streamed; cells not
/// covered by any pattern stream `default`. When both patterns cover the
/// same cell, `a_iter` wins. Patterns must be able to stream
/// `container_width * container_height` cells in total.
#[derive(Clone, Debug)]
pub struct Composite2dIter<A, B, E> {
    pub container_width: usize,
    pub container_height: usize,
    pub a_iter: A,
    pub b_iter: B,
    index: usize,
    line_a_intersection: Option<Range<usize>>,
    line_b_intersection: Option<Range<usize>>,
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
            line_a_intersection: None,
            line_b_intersection: None,
            index: 0,
            default,
        }
    }

    /// The horizontal span that `iter_box` covers on line `y`, or `None`
    /// when the pattern does not intersect that line.
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

    /// The cell content at `(x, y)`: `a_iter` when covered, else `b_iter`,
    /// else `default`.
    pub fn check_intersection_at(
        &self,
        x: usize,
        y: usize,
        a_intersect: &Option<Range<usize>>,
        b_intersect: &Option<Range<usize>>,
    ) -> Option<E>
    where
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

impl<A, B, E> Iterator2d for Composite2dIter<A, B, E> {
    fn get_width(&self) -> usize {
        self.container_width
    }

    fn get_height(&self) -> usize {
        self.container_height
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
                self.line_a_intersection = self.intersect_line(y, &self.a_iter);
                self.line_b_intersection = self.intersect_line(y, &self.b_iter);
            }

            match (&self.line_a_intersection, &self.line_b_intersection) {
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        let total = self.container_width * self.container_height;
        let remaining = total.saturating_sub(self.index);

        (remaining, Some(remaining))
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

/// One row of a [`Composite2dIter`], usable directly or through a
/// [`Flip2dIter`].
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

/// Streams the canvas row by row, alternating the scan direction on every
/// line. This matches the serpentine wiring commonly used for WS2812
/// matrices: the data flows left-to-right on one row and right-to-left on
/// the next.
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
        loop {
            let item = if self.flip {
                self.line_iter.next_back()
            } else {
                self.line_iter.next()
            };

            if item.is_some() {
                return item;
            }

            if self.current_line + 1 >= self.inner.get_height() {
                return None;
            }

            self.current_line += 1;
            self.flip = !self.flip;
            self.line_iter = self.inner.get_line_iter(self.current_line);
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining_lines_items =
            (self.inner.get_height() - self.current_line - 1) * self.inner.get_width();

        let current_line_hint = self.line_iter.size_hint();

        (
            current_line_hint.0 + remaining_lines_items,
            current_line_hint
                .1
                .map(|upper| upper + remaining_lines_items),
        )
    }
}

/// Joins patterns side by side, left to right, into one wider pattern.
///
/// All patterns must share the same height. `N` must be at least 1.
///
/// Values are consumed from the sub-patterns lazily, one row at a time, in
/// lockstep across all columns of the current sub-pattern.
#[derive(Clone, Debug)]
pub struct Chain2dIter<const N: usize, T> {
    pub inner: [T; N],
    tot_width: usize,
    counter: usize,
}

/// Error returned when chained patterns do not share the same height.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeightMismatch;

impl<const N: usize, T> Chain2dIter<N, T> {
    pub fn new(array: [T; N]) -> Result<Self, HeightMismatch>
    where
        T: Iterator2d,
    {
        let Some(first) = array.first() else {
            return Err(HeightMismatch);
        };

        let mut tot_width = first.get_width();

        for window in array.windows(2) {
            let (i, j) = (&window[0], &window[1]);

            if i.get_height() != j.get_height() {
                return Err(HeightMismatch);
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
/// Materializes an [`Iterator2d`] pattern into a fixed-size buffer of
/// `width * height` cells, providing random access ([`Buffer2d`]) without
/// re-running the source.
///
/// Cell `(x, y)` maps to `y * width + x`, matching [`Buffer2d::cell_index`].
#[derive(Clone, Debug)]
pub struct BufferedIterator2d<T> {
    inner: T,
    buffer: Box<[Color]>,
}

impl<T> BufferedIterator2d<T>
where
    T: Iterator2d + Iterator<Item = Color>,
{
    pub fn new(inner: T, default: Color) -> Self {
        let cell_count = inner.get_width() * inner.get_height();
        let mut buffer = repeat_n(default, cell_count).collect::<Box<[_]>>();
        let mut inner = inner;

        for (cell, color) in buffer.iter_mut().zip(&mut inner) {
            *cell = color;
        }

        Self { inner, buffer }
    }

    /// The whole stored frame, row-major.
    pub fn as_slice(&self) -> &[Color] {
        &self.buffer
    }
}

impl<T> Iterator2d for BufferedIterator2d<T>
where
    T: Iterator2d,
{
    fn get_width(&self) -> usize {
        self.inner.get_width()
    }

    fn get_height(&self) -> usize {
        self.inner.get_height()
    }
}

impl<T> Buffer2d for BufferedIterator2d<T>
where
    T: Iterator2d,
{
    type Item = Color;

    fn item_at(&self, x: usize, y: usize) -> Option<Self::Item> {
        self.cell_index(x, y).map(|i| self.buffer[i])
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;

    use super::*;

    /// A solid rectangle of one color, usable as canvas element both for
    /// the streaming `Iterator` path and the random access `Buffer2d` path.
    #[derive(Clone, Debug)]
    struct Solid {
        x: usize,
        y: usize,
        width: usize,
        height: usize,
        color: Color,
        remaining: usize,
    }

    impl Solid {
        fn new(x: usize, y: usize, width: usize, height: usize, color: Color) -> Self {
            Self {
                x,
                y,
                width,
                height,
                color,
                remaining: width * height,
            }
        }
    }

    impl Iterator for Solid {
        type Item = Color;

        fn next(&mut self) -> Option<Self::Item> {
            self.remaining.checked_sub(1).map(|left| {
                self.remaining = left;
                self.color
            })
        }
    }

    impl Iterator2d for Solid {
        fn get_width(&self) -> usize {
            self.width
        }

        fn get_height(&self) -> usize {
            self.height
        }
    }

    impl PositionedIterator2d for Solid {
        fn get_x(&self) -> usize {
            self.x
        }

        fn get_y(&self) -> usize {
            self.y
        }
    }

    impl Buffer2d for Solid {
        type Item = Color;

        fn item_at(&self, x: usize, y: usize) -> Option<Self::Item> {
            self.cell_index(x, y).map(|_| self.color)
        }
    }

    /// 3x2 canvas, one 1x2 blue column at x=1.
    fn blue_column_scene() -> Composite2dIter<Solid, Solid, Color> {
        Composite2dIter::new(
            3,
            2,
            Solid::new(1, 0, 1, 2, Color::DARK_BLUE),
            Solid::new(9, 9, 0, 0, Color::BLACK),
            Color::BLACK,
        )
    }

    #[test]
    fn composite_streams_full_canvas_row_major() {
        let scene = blue_column_scene();
        let frame: Vec<Color> = scene.clone().collect();

        let expected = [
            Color::BLACK,
            Color::DARK_BLUE,
            Color::BLACK,
            Color::BLACK,
            Color::DARK_BLUE,
            Color::BLACK,
        ];

        assert_eq!(frame, expected);
        assert_eq!(scene.size_hint(), (6, Some(6)));
    }

    #[test]
    fn composite_overlay_a_wins() {
        // Two overlapping red/blue blocks: the shared cell must be red.
        let scene = Composite2dIter::new(
            4,
            1,
            Solid::new(1, 0, 2, 1, Color::DARK_GREEN),
            Solid::new(2, 0, 2, 1, Color::DARK_RED),
            Color::BLACK,
        );
        let frame: Vec<Color> = scene.collect();

        assert_eq!(
            frame,
            [
                Color::BLACK,
                Color::DARK_GREEN,
                Color::DARK_GREEN,
                Color::DARK_RED,
            ]
        );
    }

    #[test]
    fn composite_line_iter_matches_frame_rows() {
        let scene = blue_column_scene();
        let frame: Vec<Color> = scene.clone().collect();
        let width = scene.get_width();

        for (y, row) in frame.chunks(width).enumerate() {
            let line: Vec<Color> = (&scene).get_line_iter(y).collect();

            assert_eq!(line, row);
        }
    }

    #[test]
    fn intersect_line_clips_to_container() {
        let scene = blue_column_scene();

        // Pattern wider than the container: the intersection is clipped.
        let clipped = scene.intersect_line(0, &Solid::new(2, 0, 5, 2, Color::BLACK));

        assert_eq!(clipped, Some(2..3));

        // Pattern fully outside the container on x.
        let outside = scene.intersect_line(0, &Solid::new(5, 0, 1, 2, Color::BLACK));

        assert_eq!(outside, None);

        // Pattern outside on y.
        let below = scene.intersect_line(5, &Solid::new(0, 9, 1, 2, Color::BLACK));

        assert_eq!(below, None);
    }

    #[test]
    fn flip_serpentine_ordering() {
        // Two-color scene so every row reads the same left-to-right:
        // row 0 and row 1 are both [G, R, R].
        let scene = Composite2dIter::new(
            3,
            2,
            Solid::new(0, 0, 1, 2, Color::DARK_GREEN),
            Solid::new(1, 0, 2, 2, Color::DARK_RED),
            Color::BLACK,
        );
        let frame: Vec<Color> = scene.clone().collect();

        assert_eq!(
            frame,
            [
                Color::DARK_GREEN,
                Color::DARK_RED,
                Color::DARK_RED,
                Color::DARK_GREEN,
                Color::DARK_RED,
                Color::DARK_RED,
            ]
        );

        // `flip = true` reads the first line right-to-left, the second one
        // left-to-right, alternating forever.
        let flipped: Vec<Color> = Flip2dIter::new(&scene, true).collect();

        assert_eq!(
            flipped,
            [
                Color::DARK_RED,
                Color::DARK_RED,
                Color::DARK_GREEN,
                Color::DARK_GREEN,
                Color::DARK_RED,
                Color::DARK_RED,
            ]
        );

        let straight: Vec<Color> = Flip2dIter::new(&scene, false).collect();

        assert_eq!(
            straight,
            [
                Color::DARK_GREEN,
                Color::DARK_RED,
                Color::DARK_RED,
                Color::DARK_RED,
                Color::DARK_RED,
                Color::DARK_GREEN,
            ]
        );
    }

    #[test]
    fn flip_size_hint_tracks_line_progress() {
        let scene = blue_column_scene();

        let mut flipped = Flip2dIter::new(&scene, false);

        assert_eq!(flipped.size_hint(), (6, Some(6)));

        for _ in 0..3 {
            flipped.next().unwrap();
        }
        assert_eq!(flipped.size_hint(), (3, Some(3)));

        for _ in 0..3 {
            flipped.next().unwrap();
        }

        assert_eq!(flipped.size_hint(), (0, Some(0)));
        assert!(flipped.next().is_none());
        assert_eq!(flipped.size_hint(), (0, Some(0)));
    }

    #[test]
    fn chain_rejects_mixed_heights() {
        let a = Solid::new(0, 0, 2, 2, Color::BLACK);
        let b = Solid::new(0, 0, 1, 3, Color::BLACK);

        assert!(Chain2dIter::new([a, b]).is_err());
    }

    #[test]
    fn chain_streams_row_major_across_slots() {
        let chain = Chain2dIter::new([
            Solid::new(0, 0, 2, 2, Color::DARK_RED),
            Solid::new(0, 0, 1, 2, Color::DARK_BLUE),
        ])
        .unwrap();

        assert_eq!((chain.get_width(), chain.get_height()), (3, 2));

        let frame: Vec<Color> = chain.clone().collect();

        assert_eq!(
            frame,
            [
                Color::DARK_RED,
                Color::DARK_RED,
                Color::DARK_BLUE,
                Color::DARK_RED,
                Color::DARK_RED,
                Color::DARK_BLUE,
            ]
        );
    }

    #[test]
    fn chain_item_at_routes_to_slots() {
        let chain = Chain2dIter::new([
            Solid::new(0, 0, 2, 2, Color::DARK_RED),
            Solid::new(0, 0, 1, 2, Color::DARK_BLUE),
        ])
        .unwrap();

        assert_eq!(chain.item_at(0, 0), Some(Color::DARK_RED));
        assert_eq!(chain.item_at(1, 1), Some(Color::DARK_RED));
        assert_eq!(chain.item_at(2, 0), Some(Color::DARK_BLUE));
        assert_eq!(chain.item_at(3, 0), None);
        assert_eq!(chain.item_at(0, 2), None);
    }

    #[test]
    fn buffered_iterator_snapshots_for_random_access() {
        let chain = Chain2dIter::new([
            Solid::new(0, 0, 2, 2, Color::DARK_RED),
            Solid::new(0, 0, 1, 2, Color::DARK_BLUE),
        ])
        .unwrap();

        let buffered = BufferedIterator2d::new(chain, Color::BLACK);

        assert_eq!(buffered.as_slice().len(), 6);

        for y in 0..buffered.get_height() {
            for x in 0..buffered.get_width() {
                let expected = if x == 2 {
                    Color::DARK_BLUE
                } else {
                    Color::DARK_RED
                };

                assert_eq!(buffered.item_at(x, y), Some(expected));
            }
        }

        assert_eq!(buffered.item_at(3, 0), None);
    }
}
