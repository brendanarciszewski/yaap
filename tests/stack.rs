use core::{fmt, ptr::NonNull};
use generic_array::{ArrayLength, GenericArray};
use typenum::{U1024, U127, U128, U16};
use yaap::{
    a::{self, Allocator},
    prelude::*,
};

mod stack_alloc {
    use super::*;

    #[repr(C, align(64))]
    struct AlignedData<N: ArrayLength<u8>>(pub GenericArray<u8, N>);

    impl<N> fmt::Debug for AlignedData<N>
    where
        N: ArrayLength<u8>,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list().entries(self.0.iter()).finish()
        }
    }

    #[derive(Debug)]
    pub struct StackResource<N: ArrayLength<u8>> {
        data: AlignedData<N>,
        used: usize,
    }

    impl<N> fmt::Display for StackResource<N>
    where
        N: ArrayLength<u8>,
    {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.debug_list()
                .entries(self.data.0[..self.used].iter())
                .finish()
        }
    }

    impl<N> MemoryResource for StackResource<N>
    where
        N: ArrayLength<u8>,
    {
        unsafe fn allocate_bytes(&mut self, size: usize, align: usize) -> a::PtrUninit<()> {
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

        unsafe fn deallocate_bytes(&mut self, _pointer: *mut (), _size: usize, _align: usize) {}
    }

    impl<N> StackResource<N>
    where
        N: ArrayLength<u8>,
    {
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
mod tracked;
use deque::Seque;
use stack_alloc::StackResource;
use tracked::Tracked;

#[test]
fn none() {
    let res = Tracked::new(StackResource::<U1024>::new());
    let _c = Seque::<usize, U16>::with_capacity_in(0, Allocator::new(res.clone()));
    assert_eq!(res.borrow().used(), 128);
    for v in res.borrow().slice() {
        assert_eq!(v, &0)
    }
}

#[test]
#[should_panic]
fn single_fail() {
    let mut c = Seque::<usize, U16>::with_capacity_in(
        1,
        Allocator::new(Tracked::new(StackResource::<U127>::new())),
    );
    // fails here because allocate returned None, but the capacity wasn't updated
    // TODO: propogate error to caller
    c.push_back(4);
}

#[test]
fn single() {
    let res = Tracked::new(StackResource::<U128>::new());
    let mut c = Seque::<usize, U16>::with_capacity_in(1, Allocator::new(res.clone()));
    c.push_back(4);
    assert_eq!(4, c[0]);
    c[0] = 5;
    assert_eq!(res.borrow().used(), 128);
    println!("{:?}", &res.borrow())
}

#[test]
fn reallocate() {
    let res = Tracked::new(StackResource::<U1024>::new());
    let mut c = Seque::<usize, U16>::with_capacity_in(1, Allocator::new(res.clone()));
    for i in 0..c.node_array_len() + 1 {
        c.push_back(2 + i);
    }
    assert_eq!(res.borrow().used(), 672);
}
#[test]
fn large_allocate() {
    let res = Tracked::new(StackResource::<U1024>::new());
    type Seq16 = Seque<usize, U16>;
    let mut c = Seq16::with_capacity_in(Seq16::NODE_ARRAY_LEN * 2, Allocator::new(res.clone()));
    for i in 0..Seq16::NODE_ARRAY_LEN + 2 {
        c.push_back(2 + i);
    }
    assert_eq!(res.borrow().used(), 672);
    println!("{}", &**res.borrow())
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn no_push() {
    let res = Tracked::new(StackResource::<U1024>::new());
    let mut c = Seque::<u8, U16>::with_capacity_in(1, Allocator::new(res.clone()));
    c[0] = 5;
}

#[test]
#[should_panic(expected = "index out of bounds")]
fn no_push_empty() {
    let res = Tracked::new(StackResource::<U1024>::new());
    let mut c = Seque::<u8, U16>::with_capacity_in(0, Allocator::new(res.clone()));
    c[0] = 5;
}

#[test]
fn test_drop() {
    static mut COUNTER: isize = 0;
    struct Test;
    impl Test {
        pub fn new() -> Self {
            unsafe { COUNTER += 1; }
            Test
        }
    }
    impl Drop for Test {
        fn drop(&mut self) {
            unsafe { COUNTER -= 1; }
        }
    }

    let res = Tracked::new(StackResource::<U1024>::new());
    type Seq16 = Seque<Test, U16>;
    {
        let mut c = Seq16::with_capacity_in(Seq16::NODE_ARRAY_LEN * 2, Allocator::new(res.clone()));
        for _i in 0..Seq16::NODE_ARRAY_LEN + 2 {
            c.push_back(Test::new());
        }
        assert_eq!(res.borrow().used(), 32);
        assert_eq!(unsafe {COUNTER}, 18);
    }
    assert_eq!(unsafe {COUNTER}, 0);
}
