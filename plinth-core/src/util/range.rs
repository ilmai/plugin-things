use std::{ops::{Range, RangeBounds, Bound, Add}, fmt::Display};

use num_traits::AsPrimitive;

pub fn range_from_bounds<T, R>(range: R, len: usize) -> Range<T>
where
    T: Add<Output = T> + Copy + Display + PartialOrd + 'static,
    R: RangeBounds<T>,
    usize: AsPrimitive<T>,
{
    let (start, end) = range_from_bounds_impl(range, len);

    assert!(start <= len.as_(), "range start index {start} out of range of length {len}");
    assert!(end <= len.as_(), "range end index {end} out of range of length {len}");

    Range { start, end }
}

pub fn trimmed_range_from_bounds<T, R>(range: R, len: usize) -> Range<T>
where
    T: Add<Output = T> + Copy + Display + PartialOrd + AsPrimitive<usize> + 'static,
    R: RangeBounds<T>,
    usize: AsPrimitive<T>,
{
    let (mut start, mut end) = range_from_bounds_impl(range, len);

    start = usize::min(start.as_(), len).as_();
    end = usize::min(end.as_(), len).as_();

    Range { start, end }
}


pub fn range_from_bounds_impl<T, R>(range: R, len: usize) -> (T, T)
where
    T: Add<Output = T> + Copy + Display + PartialOrd + 'static,
    R: RangeBounds<T>,
    usize: AsPrimitive<T>,
{
    let start = match range.start_bound() {
        Bound::Included(index) => *index,
        Bound::Excluded(index) => *index + 1.as_(),
        Bound::Unbounded => 0.as_(),
    };

    let end = match range.end_bound() {
        Bound::Included(index) => *index + 1.as_(),
        Bound::Excluded(index) => *index,
        Bound::Unbounded => len.as_(),
    };

    (start, end)
}
