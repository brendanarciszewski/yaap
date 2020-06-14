#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

pub mod prelude {
    pub use super::a::Allocator;
    #[cfg(feature = "alloc")]
    pub use super::a::AllocatorAwareContainer;
}

pub mod a {
    #[cfg(feature = "alloc")]
    use alloc::rc::Rc;
    use core::{cell::RefCell, mem, mem::MaybeUninit, ptr::NonNull};

    #[cfg(feature = "alloc")]
    pub trait AllocatorAwareContainer {
        fn allocator(&self) -> Rc<RefCell<dyn Allocator>>;
    }

    pub type Ptr<T> = Option<NonNull<T>>;
    pub type PtrUninit<T> = Ptr<MaybeUninit<T>>;

    pub trait Allocator {
        unsafe fn allocate(&mut self, size: usize, align: usize) -> PtrUninit<()>;
        unsafe fn deallocate(&mut self, pointer: *mut (), size: usize, align: usize);
    }

    pub unsafe fn allocate<T>(alloc: &RefCell<dyn Allocator>, num_objects: usize) -> PtrUninit<T> {
        alloc
            .borrow_mut()
            .allocate(mem::size_of::<T>() * num_objects, mem::align_of::<T>())
            .map(NonNull::cast::<MaybeUninit<T>>)
    }

    pub unsafe fn deallocate<T>(
        alloc: &RefCell<dyn Allocator>,
        pointer: *mut T,
        num_objects: usize,
    ) {
        alloc.borrow_mut().deallocate(
            pointer as *mut (),
            mem::size_of::<T>() * num_objects,
            mem::align_of::<T>(),
        )
    }
}
