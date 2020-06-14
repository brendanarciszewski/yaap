use core::{cell::RefCell, fmt, ptr::NonNull};
use std::rc::Rc;
use yaap::{a, prelude::*};
use typenum::{U1024, U127, U128};
use generic_array::{ArrayLength, GenericArray};

mod stack_alloc {
    use super::*;

    #[repr(C, align(64))]
    struct AlignedData<N: ArrayLength<u8>>(pub GenericArray<u8, N>);

    impl<N> fmt::Debug for AlignedData<N> where N: ArrayLength<u8> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.0.iter()).finish()
        }
    }

    #[derive(Debug)]
    pub struct StackAllocator<N: ArrayLength<u8>> {
        data: AlignedData<N>,
        used: usize,
    }

    impl<N> fmt::Display for StackAllocator<N> where N: ArrayLength<u8> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list()
                .entries(self.data.0[..self.used].iter())
                .finish()
        }
    }

    impl<N> Allocator for StackAllocator<N> where N: ArrayLength<u8> {
        unsafe fn allocate(&mut self, size: usize, align: usize) -> a::PtrUninit<()> {
            if self.used + size > N::USIZE {
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

    impl<N> StackAllocator<N> where N: ArrayLength<u8>{
        pub fn new() -> Self {
            Self {
                data: AlignedData(GenericArray::<u8, N>::default()), //unsafe {mem::MaybeUninit::uninit().assume_init()},
                used: 0,
            }
        }

        pub fn used(&self) -> usize {
            self.used
        }

        pub fn slice(&self) -> &[u8] {
            &self.data.0[..]
        }
    }
}

mod deque;

use deque::Seque;
use stack_alloc::StackAllocator;

#[test]
fn none() {
    let s_alloc = Rc::new(RefCell::new(StackAllocator::<U1024>::new()));
    let _c = Seque::<usize>::with_capacity_in(0, s_alloc.clone());
    assert_eq!(s_alloc.borrow().used(), 128);
    for v in s_alloc.borrow().slice() {
        assert_eq!(v, &0)
    }
}

#[test]
#[should_panic]
fn single_fail() {
    let s_alloc = Rc::new(RefCell::new(StackAllocator::<U127>::new()));
    let mut c = Seque::<usize>::with_capacity_in(1, s_alloc.clone());
    // fails here because allocate returned None, but the capacity wasn't updated
    // TODO: propogate error to caller
    c.push_back(4);
}

#[test]
fn single() {
    let s_alloc = Rc::new(RefCell::new(StackAllocator::<U128>::new()));
    let mut c = Seque::<usize>::with_capacity_in(1, s_alloc.clone());
    c.push_back(4);
    assert_eq!(4, c[0]);
    c[0] = 5;
    assert_eq!(s_alloc.borrow().used(), 128);
    println!("{:?}", &s_alloc.borrow())
}

#[test]
fn reallocate() {
    let s_alloc = Rc::new(RefCell::new(StackAllocator::<U1024>::new()));
    let mut c = Seque::<usize>::with_capacity_in(1, s_alloc.clone());
    for i in 0..deque::NODE_ARRAY_LEN + 1 {
        c.push_back(2 + i);
    }
    assert_eq!(s_alloc.borrow().used(), 672);
}
#[test]
fn large_allocate() {
    let s_alloc = Rc::new(RefCell::new(StackAllocator::<U1024>::new()));
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
    let s_alloc = Rc::new(RefCell::new(StackAllocator::<U1024>::new()));
    let mut c = Seque::<u8>::with_capacity_in(1, s_alloc.clone());
    c[0] = 5;
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn no_push_empty() {
    let s_alloc = Rc::new(RefCell::new(StackAllocator::<U1024>::new()));
    let mut c = Seque::<u8>::with_capacity_in(0, s_alloc.clone());
    c[0] = 5;
}
