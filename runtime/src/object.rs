use std::{
    fmt::Display,
    ops::{Deref, DerefMut},
    ptr,
};

mod string;

macro_rules! register_object_impl {
    () => {};
}

pub(crate) struct ObjectMeta {}

pub(crate) enum Object {}

pub(crate) struct ManagedReference<T> {
    ptr: *mut T,
}

impl<T> ManagedReference<T> {
    pub unsafe fn new(ptr: *mut T) -> Self {
        Self { ptr }
    }

    pub fn as_ptr(&self) -> *const T {
        self.ptr
    }

    pub fn as_mut(&mut self) -> *mut T {
        self.ptr
    }
}

impl<T> Clone for ManagedReference<T> {
    fn clone(&self) -> Self {
        Self { ptr: self.ptr }
    }
}

impl<T> Deref for ManagedReference<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<T> DerefMut for ManagedReference<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.ptr }
    }
}

impl<T> PartialEq for ManagedReference<T> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.ptr, other.ptr)
    }
}

impl<T> Eq for ManagedReference<T> {}

impl<T> Display for ManagedReference<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<object at {:#x}>", self.ptr as usize)
    }
}
