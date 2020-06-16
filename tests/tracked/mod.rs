use core::{cell::RefCell, ops::Deref};
use std::rc::Rc;
use yaap::{a, prelude::*};

#[derive(Debug)]
pub struct Tracked<T>(T, usize);

impl<T> MemoryResource for Tracked<T>
where
    T: MemoryResource,
{
    unsafe fn allocate_bytes(&mut self, size: usize, align: usize) -> a::PtrUninit<()> {
        let p = self.0.allocate_bytes(size, align);
        if p.is_some() {
            self.1 += size;
        }
        p
    }

    unsafe fn deallocate_bytes(&mut self, pointer: *mut (), size: usize, align: usize) {
        if !pointer.is_null() {
            self.1 -= size;
        }
        self.0.deallocate_bytes(pointer, size, align)
    }
}

impl<T> Drop for Tracked<T> {
    fn drop(&mut self) {
        assert_eq!(self.count(), 0);
    }
}

impl<T> Tracked<T>
where
    T: MemoryResource,
{
    pub fn new(inner: T) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Tracked(inner, 0)))
    }
}

impl<T> Deref for Tracked<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> Tracked<T> {
    pub fn count(&self) -> usize {
        self.1
    }
}
