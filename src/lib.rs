extern crate alloc;

pub mod prelude {
    pub use super::a::AllocatorAwareContainer;
    pub use super::a::MemoryResource;
}

pub mod a {
    use alloc::rc::Rc;
    use core::{
        cell::{Ref, RefCell},
        mem,
        mem::MaybeUninit,
        ptr::NonNull,
    };

    pub trait AllocatorAwareContainer {
        fn allocator(&self) -> Allocator;
    }

    pub type Ptr<T> = Option<NonNull<T>>;
    pub type PtrUninit<T> = Ptr<MaybeUninit<T>>;

    pub trait MemoryResource {
        unsafe fn allocate_bytes(&mut self, size: usize, align: usize) -> PtrUninit<()>;
        unsafe fn deallocate_bytes(&mut self, pointer: *mut (), size: usize, align: usize);
    }

    /// # A pointer to a memory resource
    ///
    /// Requires that the resource is Send so that the allocator can be sent
    #[derive(Clone)]
    pub struct Allocator(Rc<RefCell<dyn MemoryResource + Send>>);

    impl Allocator {
        pub fn new(resource: Rc<RefCell<dyn MemoryResource + Send>>) -> Self {
            Self(resource)
        }

        pub unsafe fn allocate<T>(&self, num_objects: usize) -> PtrUninit<T> {
            self.allocate_bytes(mem::size_of::<T>() * num_objects, mem::align_of::<T>())
                .map(NonNull::cast::<MaybeUninit<T>>)
        }

        pub unsafe fn deallocate<T>(&self, pointer: *mut T, num_objects: usize) {
            self.deallocate_bytes(
                pointer as *mut (),
                mem::size_of::<T>() * num_objects,
                mem::align_of::<T>(),
            )
        }

        pub unsafe fn allocate_bytes(&self, size: usize, align: usize) -> PtrUninit<()> {
            self.0.borrow_mut().allocate_bytes(size, align)
        }

        pub unsafe fn deallocate_bytes(&self, pointer: *mut (), size: usize, align: usize) {
            self.0.borrow_mut().deallocate_bytes(pointer, size, align)
        }

        pub fn get(&self) -> Ref<dyn MemoryResource> {
            self.0.borrow()
        }
    }
}
