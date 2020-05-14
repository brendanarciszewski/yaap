use yaap::*;
use core::{mem, ops};

#[repr(align(64))]
struct AlignedData([u8; 1024]);

struct StackAllocator {
    data: AlignedData,
    used: usize,
}

impl Allocator for StackAllocator {
    unsafe fn allocate(&mut self, size: usize, align: usize) -> *mut () {
        let start_ptr = self.data.0.as_mut_ptr();
        let ptr = {
            let ptr = start_ptr.add(self.used);
            let offset = ptr.align_offset(align);
            ptr.add(offset)
        };
        self.used += ptr as usize - start_ptr as usize; // ptr.offset_from(start_ptr)
        self.used += size;
        ptr as *mut _
    }

    unsafe fn deallocate(&mut self, _pointer: *mut (), _size: usize, _align: usize) {}
}

impl StackAllocator {
    pub fn new() -> Self {
        Self {
            data: unsafe {mem::MaybeUninit::uninit().assume_init()},
            used: 0,
        }
    }
}

struct CustomDeque<'a, T> {
    length: usize,
    capacity: usize,
    data: *mut T,
    alloc: &'a mut dyn Allocator
}

impl<T> AllocatorAwareContainer for CustomDeque<'_, T> {
    fn allocator(&mut self) -> &mut dyn Allocator {
        self.alloc
    }
}

impl<'a, T> CustomDeque<'a, T> {
    pub fn with_capacity(capacity: usize, alloc: &'a mut dyn Allocator) -> Self {
        let data = helper::allocate_object::<T>(alloc, capacity);
        Self {
            length: 0,
            capacity,
            data,
            alloc,
        }
    }

    pub fn push_back(&mut self, val: T) {
        if self.length >= self.capacity {
            todo!("Reallocate self.data with larger capacity and forget old");
        }
        unsafe {
            self.data.add(self.length).write(val)
        }
        self.length += 1;
    }
}

impl<'a, T> ops::Index<usize> for CustomDeque<'a, T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.length {
            panic!("Attempt to access unintialized memory");
        }
        unsafe {&*self.data.add(index)}
    }
}

#[test]
fn it_works() {
    let mut alloc = StackAllocator::new();
    let mut c = CustomDeque::<'_, u16>::with_capacity(1, &mut alloc);
    c.push_back(4);
    assert_eq!(2 + 2, c[0]);
}
