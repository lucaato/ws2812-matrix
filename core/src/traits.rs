/// The extents of a pattern that can be streamed row by row.
pub trait Iterator2d {
    /// Width of the pattern in cells.
    fn get_width(&self) -> usize;

    /// Height of the pattern in cells.
    fn get_height(&self) -> usize;

    /// Maps a linear index into the pattern to `(x, y)` coordinates, or
    /// `None` when the index is past the end.
    #[inline]
    fn get_coordinates(&self, i: usize) -> Option<(usize, usize)> {
        let y = i / self.get_width();

        (y < self.get_height()).then(|| {
            let x = i % self.get_width();

            (x, y)
        })
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

/// A pattern that sits at a specific place inside a bigger canvas.
pub trait PositionedIterator2d: Iterator2d {
    /// X coordinate of the top-left cell inside the canvas.
    fn get_x(&self) -> usize;
    /// Y coordinate of the top-left cell inside the canvas.
    fn get_y(&self) -> usize;
}

/// Random access to the cells of an [`Iterator2d`] pattern.
pub trait Buffer2d: Iterator2d {
    type Item;

    /// Maps `(x, y)` to the linear index of the cell, or `None` when out of
    /// bounds.
    #[inline]
    fn cell_index(&self, x: usize, y: usize) -> Option<usize> {
        (x < self.get_width() && y < self.get_height()).then(|| y * self.get_width() + x)
    }

    /// The cell at `(x, y)`, or `None` when out of bounds.
    fn item_at(&self, x: usize, y: usize) -> Option<Self::Item>;
}

/// A pattern whose rows can be pulled one at a time.
pub trait LineIter2d {
    type Item;
    type Iter: Iterator<Item = Self::Item>;

    /// Returns an iterator over the cells of `line`.
    fn get_line_iter(self, line: usize) -> Self::Iter;
}
