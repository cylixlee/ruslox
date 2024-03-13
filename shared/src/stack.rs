use std::{
    mem::{self, ManuallyDrop},
    ops::{Index, IndexMut},
    ptr,
};

use crate::error::{ErrorItem, InterpretError, InterpretResult};

const DEFAULT_STACK_CAPACITY: usize = u8::MAX as usize + 1;

pub struct Stack<T, const N: usize = DEFAULT_STACK_CAPACITY> {
    data: [ManuallyDrop<T>; N],
    top: usize,
}

impl<T, const N: usize> Stack<T, N> {
    pub fn new() -> Self {
        Self {
            data: unsafe { mem::zeroed() },
            top: 0,
        }
    }

    pub fn push(&mut self, value: T) -> InterpretResult {
        if self.top >= N {
            return Err(InterpretError::Simple(
                ErrorItem::error()
                    .with_code("E1001")
                    .with_message("stack overflow"),
            ));
        }
        unsafe {
            ptr::write(&mut self.data[self.top], ManuallyDrop::new(value));
        }
        self.top += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> InterpretResult<T> {
        if self.is_empty() {
            return Err(InterpretError::Simple(
                ErrorItem::error()
                    .with_code("E1002")
                    .with_message("stack underflow"),
            ));
        }
        self.top -= 1;
        let slot = unsafe { ptr::read(&mut self.data[self.top]) };
        Ok(ManuallyDrop::into_inner(slot))
    }

    pub fn peek(&self) -> Option<&T> {
        if self.top > 0 {
            Some(&self.data[self.top - 1])
        } else {
            None
        }
    }

    pub fn len(&self) -> usize {
        self.top
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn clear(&mut self) {
        while !self.is_empty() {
            mem::drop(self.pop().unwrap());
        }
    }
}

impl<T, const N: usize> Drop for Stack<T, N> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T, const N: usize> Index<usize> for Stack<T, N> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.top {
            panic!("Index {} out of stack size {}", index, self.len());
        }
        &self.data[index]
    }
}

impl<T, const N: usize> IndexMut<usize> for Stack<T, N> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.top {
            panic!("Index {} out of stack size {}", index, self.len());
        }
        &mut self.data[index]
    }
}
