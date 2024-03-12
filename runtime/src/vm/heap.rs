use std::collections::HashMap;

use crate::object::{FromUnmanaged, GarbageCollect, ManagedReference, StringObject};

pub struct Heap {
    references: Vec<ManagedReference>,
    interned_strings: HashMap<StringObject, ManagedReference>,
}

impl Heap {
    pub fn new() -> Self {
        Self {
            references: Vec::new(),
            interned_strings: HashMap::new(),
        }
    }

    pub fn manage_string(&mut self, string: String) -> ManagedReference {
        match self.interned_strings.get(&string) {
            Some(reference) => reference.clone(),
            None => {
                let reference =
                    ManagedReference::from_unmanaged(StringObject::from(string.clone()), self);
                self.interned_strings.insert(string, reference.clone());
                reference
            }
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
