//! A tiny fixed-capacity vector for `Copy` types, so the audio thread never
//! allocates.

#[derive(Clone, Copy, Debug)]
pub struct FixedVec<T: Copy + Default, const N: usize> {
    items: [T; N],
    len: usize,
}

impl<T: Copy + Default, const N: usize> Default for FixedVec<T, N> {
    fn default() -> Self {
        Self {
            items: [T::default(); N],
            len: 0,
        }
    }
}

impl<T: Copy + Default, const N: usize> FixedVec<T, N> {
    pub fn new() -> Self {
        Self::default()
    }
    /// Returns false (and drops `v`) when full.
    pub fn push(&mut self, v: T) -> bool {
        if self.len == N {
            return false;
        }
        self.items[self.len] = v;
        self.len += 1;
        true
    }
    pub fn insert(&mut self, i: usize, v: T) -> bool {
        if self.len == N || i > self.len {
            return false;
        }
        self.items.copy_within(i..self.len, i + 1);
        self.items[i] = v;
        self.len += 1;
        true
    }
    pub fn remove(&mut self, i: usize) -> T {
        let v = self.items[i];
        self.items.copy_within(i + 1..self.len, i);
        self.len -= 1;
        v
    }
    /// Remove the first `n` items.
    pub fn drain_front(&mut self, n: usize) {
        let n = n.min(self.len);
        self.items.copy_within(n..self.len, 0);
        self.len -= n;
    }
    pub fn retain(&mut self, mut f: impl FnMut(&T) -> bool) {
        let mut w = 0;
        for r in 0..self.len {
            if f(&self.items[r]) {
                self.items[w] = self.items[r];
                w += 1;
            }
        }
        self.len = w;
    }
    pub fn clear(&mut self) {
        self.len = 0;
    }
    pub fn len(&self) -> usize {
        self.len
    }
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    pub fn is_full(&self) -> bool {
        self.len == N
    }
    pub fn as_slice(&self) -> &[T] {
        &self.items[..self.len]
    }
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.as_slice().iter()
    }
}

impl<T: Copy + Default + PartialEq, const N: usize> PartialEq for FixedVec<T, N> {
    fn eq(&self, other: &Self) -> bool {
        self.as_slice() == other.as_slice()
    }
}
