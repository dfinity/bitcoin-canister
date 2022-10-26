use std::iter::Peekable;

/// An iterator that consumes multiple iterators and returns their items interleaved in sorted order.
/// The iterators themselves must be sorted.
pub struct MultiIter<T, A: Iterator<Item = T>, B: Iterator<Item = T>> {
    a: Peekable<A>,
    b: Peekable<B>,
}

impl<T, A: Iterator<Item = T>, B: Iterator<Item = T>> MultiIter<T, A, B> {
    pub fn new(a: A, b: B) -> Self {
        Self {
            a: a.peekable(),
            b: b.peekable(),
        }
    }
}

impl<T: PartialOrd, A: Iterator<Item = T>, B: Iterator<Item = T>> Iterator for MultiIter<T, A, B> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let next_a = self.a.peek();
        let next_b = self.b.peek();

        match (next_a, next_b) {
            (Some(next_a), Some(next_b)) => {
                if next_a < next_b {
                    self.a.next()
                } else {
                    self.b.next()
                }
            }
            (Some(_), None) => self.a.next(),
            (None, Some(_)) => self.b.next(),
            (None, None) => None,
        }
    }
}
