use std::mem;

use crate::object::ManagedReference;

pub(crate) struct Heap<T> {
    data: Vec<ManagedReference<T>>,
}

impl<T> Heap<T> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn manage(&mut self, boxed: Box<T>) -> ManagedReference<T> {
        let managed = unsafe { ManagedReference::new(Box::into_raw(boxed)) };
        self.data.push(managed.clone());
        managed
    }
}

impl<T> Drop for Heap<T> {
    fn drop(&mut self) {
        while let Some(mut reference) = self.data.pop() {
            mem::drop(unsafe { Box::from_raw(reference.as_mut()) })
        }
    }
}
