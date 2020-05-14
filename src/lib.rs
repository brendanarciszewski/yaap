#![no_std]
use core::mem;

pub trait AllocatorAwareContainer<'a> {
    //fn allocator(&self) -> &dyn Allocator;
    fn set_allocator(&mut self, alloc: &'a mut dyn Allocator);
}
pub trait Allocator {
    unsafe fn allocate(&mut self, size: usize, align: usize) -> *mut ();
    unsafe fn deallocate(&mut self, pointer: *mut (), size: usize, align: usize);
}

pub mod helper {
    use super::*;

    pub fn allocate_object<T>(alloc: &mut dyn Allocator, num_objects: usize) -> *mut T {
        unsafe {
            alloc.allocate(
                mem::size_of::<T>() * num_objects,
                mem::align_of::<T>(),
            ) as *mut _
        }
    }

    pub unsafe fn deallocate_object<T>(alloc: &mut dyn Allocator, pointer: *mut (), num_objects: usize) {
        alloc.deallocate(
            pointer,
            mem::size_of::<T>() * num_objects,
            mem::align_of::<T>(),
        )
    }
}
