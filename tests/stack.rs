use core::{cell::RefCell, fmt, ptr::NonNull};
use std::rc::Rc;
use yaap::{a, prelude::*};

mod stack_alloc {
    use super::*;
    const S_ALLOC_LEN: usize = 1024;
    #[repr(C, align(64))]
    struct AlignedData(pub [u8; S_ALLOC_LEN]);

    impl fmt::Debug for AlignedData {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.0.iter()).finish()
        }
    }

    #[derive(Debug)]
    pub struct StackAllocator {
        data: AlignedData,
        used: usize,
    }

    impl fmt::Display for StackAllocator {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list()
                .entries(self.data.0[..self.used].iter())
                .finish()
        }
    }

    impl Allocator for StackAllocator {
        unsafe fn allocate(&mut self, size: usize, align: usize) -> a::PtrUninit<()> {
            if self.used + size > S_ALLOC_LEN {
                return None;
            }
            let start_ptr = self.data.0.as_mut_ptr();
            let ptr = {
                let ptr = start_ptr.add(self.used);
                let offset = ptr.align_offset(align);
                ptr.add(offset)
            };
            self.used += ptr as usize - start_ptr as usize; // ptr.offset_from(start_ptr)
            self.used += size;
            Some(NonNull::new_unchecked(ptr as *mut _))
        }

        unsafe fn deallocate(&mut self, _pointer: *mut (), _size: usize, _align: usize) {}
    }

    impl StackAllocator {
        pub const fn new() -> Self {
            Self {
                data: AlignedData([0; 1024]), //unsafe {mem::MaybeUninit::uninit().assume_init()},
                used: 0,
            }
        }

        pub const fn used(&self) -> usize {
            self.used
        }

        pub fn slice(&self) -> &[u8] {
            &self.data.0[..]
        }
    }
}

mod deque;
impl<T> AllocatorAwareContainer for deque::Seque<T> {
    fn allocator(&self) -> Rc<RefCell<dyn Allocator>> {
        self.alloc.clone()
    }
}

use deque::Seque;
use stack_alloc::StackAllocator;

#[test]
fn none() {
    let s_alloc = Rc::new(RefCell::new(StackAllocator::new()));
    let _c = Seque::<usize>::with_capacity_in(0, s_alloc.clone());
    assert_eq!(s_alloc.borrow().used(), 128);
    for v in s_alloc.borrow().slice() {
        assert_eq!(v, &0)
    }
}

#[test]
fn single() {
    let s_alloc = Rc::new(RefCell::new(StackAllocator::new()));
    let mut c = Seque::<usize>::with_capacity_in(1, s_alloc.clone());
    c.push_back(4);
    assert_eq!(4, c[0]);
    c[0] = 5;
    assert_eq!(s_alloc.borrow().used(), 128);
    println!("{:?}", &s_alloc.borrow())
}

#[test]
fn reallocate() {
    let s_alloc = Rc::new(RefCell::new(StackAllocator::new()));
    let mut c = Seque::<usize>::with_capacity_in(1, s_alloc.clone());
    for i in 0..deque::NODE_ARRAY_LEN + 1 {
        c.push_back(2 + i);
    }
    assert_eq!(s_alloc.borrow().used(), 672);
}
#[test]
fn large_allocate() {
    let s_alloc = Rc::new(RefCell::new(StackAllocator::new()));
    let mut c = Seque::<usize>::with_capacity_in(deque::NODE_ARRAY_LEN * 2, s_alloc.clone());
    for i in 0..deque::NODE_ARRAY_LEN + 2 {
        c.push_back(2 + i);
    }
    assert_eq!(s_alloc.borrow().used(), 672);
    println!("{}", &s_alloc.borrow())
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn no_push() {
    let s_alloc = Rc::new(RefCell::new(StackAllocator::new()));
    let mut c = Seque::<u8>::with_capacity_in(1, s_alloc.clone());
    c[0] = 5;
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn no_push_empty() {
    let s_alloc = Rc::new(RefCell::new(StackAllocator::new()));
    let mut c = Seque::<u8>::with_capacity_in(0, s_alloc.clone());
    c[0] = 5;
}
