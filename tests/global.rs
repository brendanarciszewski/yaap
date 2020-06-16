use core::{mem::MaybeUninit, ptr::NonNull};
use std::alloc;
use yaap::{
    a::{self, Allocator},
    prelude::*,
};

struct Global;
impl MemoryResource for Global {
    unsafe fn allocate_bytes(&mut self, size: usize, align: usize) -> a::PtrUninit<()> {
        NonNull::new(
            alloc::alloc(alloc::Layout::from_size_align_unchecked(size, align))
                as *mut MaybeUninit<()>,
        )
    }
    unsafe fn deallocate_bytes(&mut self, pointer: *mut (), size: usize, align: usize) {
        alloc::dealloc(
            pointer as *mut u8,
            alloc::Layout::from_size_align_unchecked(size, align),
        )
    }
}

mod deque;
mod tracked;
use deque::Seque;
use tracked::Tracked;

#[test]
fn none() {
    let res = Tracked::new(Global);
    let _c = Seque::<usize>::with_capacity_in(0, Allocator::new(res));
}

#[test]
fn single() {
    let res = Tracked::new(Global);
    let mut c = Seque::<usize>::with_capacity_in(1, Allocator::new(res));
    c.push_back(4);
    assert_eq!(4, c[0]);
    c[0] = 5;
}

#[test]
fn reallocate() {
    let res = Tracked::new(Global);
    let mut c = Seque::<usize>::with_capacity_in(1, Allocator::new(res));
    for i in 0..deque::NODE_ARRAY_LEN + 1 {
        c.push_back(2 + i);
    }
}
#[test]
fn large_allocate() {
    let res = Tracked::new(Global);
    let mut c = Seque::<usize>::with_capacity_in(deque::NODE_ARRAY_LEN * 2, Allocator::new(res));
    for i in 0..deque::NODE_ARRAY_LEN + 2 {
        c.push_back(2 + i);
    }
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn no_push() {
    let res = Tracked::new(Global);
    let mut c = Seque::<u8>::with_capacity_in(1, Allocator::new(res));
    c[0] = 5;
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn no_push_empty() {
    let res = Tracked::new(Global);
    let mut c = Seque::<u8>::with_capacity_in(0, Allocator::new(res));
    c[0] = 5;
}
