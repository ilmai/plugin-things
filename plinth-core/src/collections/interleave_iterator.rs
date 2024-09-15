use std::iter::zip;

const MAX_ITERATORS: usize = 4;

pub struct InterleaveIterator<T, I: Iterator<Item = T>> {
    iterators: [Option<I>; MAX_ITERATORS],
    iterator_count: usize,
    next_iterator_index: usize,
}

impl<T, I> InterleaveIterator<T, I>
where
    I: Iterator<Item = T>
{
    pub fn new<II: IntoIterator<Item = I>>(iterators: II) -> InterleaveIterator<T, I> {
        let mut iterator_array: [Option<I>; MAX_ITERATORS] = Default::default();

        let mut iterator_count = 0;
        for (it, array_it) in zip(iterators.into_iter(), iterator_array.iter_mut()) {
            *array_it = Some(it);
            iterator_count += 1;
        }

        InterleaveIterator {
            iterators: iterator_array,
            iterator_count,
            next_iterator_index: 0
        }
    }
}

impl<T, I: Iterator<Item = T>> Iterator for InterleaveIterator<T, I> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iterator_count == 0 {
            return None;
        }

        let iterator = self.iterators[self.next_iterator_index].as_mut().unwrap();
        let result = iterator.next();
        
        self.next_iterator_index += 1;
        if self.next_iterator_index >= self.iterator_count {
            self.next_iterator_index = 0;
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::InterleaveIterator;

    #[test]
    fn empty() {
        let mut iterator = InterleaveIterator::new([[0u8; 0].iter()]);
        assert_eq!(iterator.next(), None);
    }

    #[test]
    fn iterate() {
        let a = [1.0, 2.0, 3.0];
        let b = [4.0, 5.0, 6.0];
        let c = [7.0, 8.0, 9.0];

        let iterator = InterleaveIterator::new([a.iter(), b.iter(), c.iter()]);

        assert_eq!(iterator.copied().collect::<Vec<_>>(), [1.0, 4.0, 7.0, 2.0, 5.0, 8.0, 3.0, 6.0, 9.0]);
    }

    #[test]
    fn iterate_mut() {
        let mut a = [1.0, 2.0, 3.0];
        let mut b = [4.0, 5.0, 6.0];
        let mut c = [7.0, 8.0, 9.0];

        let iterator = InterleaveIterator::new([a.iter_mut(), b.iter_mut(), c.iter_mut()]);

        for (i, value) in iterator.enumerate() {
            if i % 2 == 0 {
                *value *= 10.0;
            }
        }

        assert_eq!(a, [10.0, 2.0, 30.0]);
        assert_eq!(b, [4.0, 50.0, 6.0]);
        assert_eq!(c, [70.0, 8.0, 90.0]);
    }
}
