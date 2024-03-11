use crate::object::{GarbageCollect, ManagedReference};

pub struct Heap {
    references: Vec<ManagedReference>,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            references: Vec::new(),
        }
    }
}

impl GarbageCollect for Heap {
    fn register(&mut self, reference: ManagedReference) {
        self.references.push(reference);
    }
}

impl Drop for Heap {
    fn drop(&mut self) {
        while let Some(reference) = self.references.pop() {
            unsafe { reference.finalize() }
        }
    }
}
