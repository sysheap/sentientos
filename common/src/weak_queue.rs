extern crate alloc;

use alloc::{
    collections::VecDeque,
    sync::{Arc, Weak},
};

/// A data structure which keeps a list of weak references
/// When iterating over it, it tries to upgrade the reference.
/// If successful, the reference is returned. If not, it is removed.
pub struct WeakQueue<T> {
    queue: VecDeque<Weak<T>>,
}

impl<T> Default for WeakQueue<T> {
    fn default() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }
}

impl<T> WeakQueue<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, reference: Weak<T>) {
        self.queue.push_back(reference);
    }

    pub fn readonly_iter(&self) -> ReadonlyIter<'_, T> {
        ReadonlyIter {
            original_iter: self.queue.iter(),
        }
    }

    pub fn iter(&mut self) -> Iter<'_, T> {
        let left = self.queue.len();
        Iter { weak: self, left }
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct Iter<'a, T> {
    weak: &'a mut WeakQueue<T>,
    left: usize,
}

impl<T> Iterator for Iter<'_, T> {
    type Item = Arc<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.left == 0 {
            return None;
        }

        while let Some(reference) = self.weak.queue.pop_front() {
            self.left -= 1;
            if let Some(strong) = reference.upgrade() {
                self.weak.add(reference);
                return Some(strong);
            }
            if self.left == 0 {
                break;
            }
        }

        None
    }
}

pub struct ReadonlyIter<'a, T> {
    original_iter: alloc::collections::vec_deque::Iter<'a, Weak<T>>,
}

impl<T> Iterator for ReadonlyIter<'_, T> {
    type Item = Arc<T>;

    fn next(&mut self) -> Option<Self::Item> {
        for reference in self.original_iter.by_ref() {
            if let Some(reference) = reference.upgrade() {
                return Some(reference);
            }
        }
        None
    }
}
