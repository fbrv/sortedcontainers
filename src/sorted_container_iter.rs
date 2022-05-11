use std::iter::FusedIterator;

pub struct SortedContainerIter<'a, T: Clone + Ord> {
    pub(crate) pos: usize,
    pub(crate) idx: usize,
    pub(crate) data: &'a Vec<Vec<T>>,
}

impl<'a, T: Clone + Ord> Iterator for SortedContainerIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.idx >= self.data[self.pos].len() {
            self.pos += 1;
            self.idx = 0;
        }
        if self.pos >= self.data.len() {
            return None;
        }
        self.idx += 1;
        Some(&self.data[self.pos][self.idx - 1])
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let mut max = 0;
        for vec in self.data {
            max += vec.len();
        }
        (0, Some(max))
    }
}
impl<T: Ord + Clone> FusedIterator for SortedContainerIter<'_, T> {}
