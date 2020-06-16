use core::{marker::PhantomData, ops, ptr::NonNull};
use typenum::Unsigned;
use yaap::a::{self, Allocator};

type Data<T> = a::Ptr<T>;
type Link<T, N> = Data<Node<T, N>>;

pub(crate) struct Node<T, N> {
    data: Data<T>,
    next: Link<T, N>,
    _p: PhantomData<N>,
}

impl<T, N> Node<T, N>
where
    N: Unsigned,
{
    /// assert!(N::USIZE < isize::MAX);
    const ARRAY_LEN: usize = N::USIZE;

    pub fn with_data(alloc: &Allocator) -> Self {
        Self {
            data: Self::allocate_node_data(alloc),
            next: None,
            _p: PhantomData,
        }
    }

    fn allocate_node_data(alloc: &Allocator) -> Data<T> {
        // SAFETY: data is never accessed before first writing a valid value
        unsafe { alloc.allocate::<T>(Self::ARRAY_LEN).map(NonNull::cast) }
    }

    fn allocate_next_node(alloc: &Allocator) -> Link<T, N> {
        // Safety: Able to remove MaybeUninit because underlying data is initialized
        unsafe {
            alloc.allocate::<Node<T, N>>(1).map(|mut node| {
                node.as_mut().as_mut_ptr().write(Self::with_data(alloc));
                node.cast()
            })
        }
    }

    pub fn allocate_node_chain(&mut self, amount: usize, alloc: &Allocator) {
        let mut node = self;
        while let Some(ref mut n) = node.next {
            node = unsafe { n.as_mut() };
        }
        for _i in 0..amount {
            node.next = Self::allocate_next_node(alloc);
            node = node.next_node_mut().expect("Just allocated");
        }
    }

    unsafe fn deallocate_node_chain(&mut self, alloc: &Allocator) {
        if let Some(next) = self.next_node_mut() {
            // Safety: any children nodes of a parent node were also allocated
            next.deallocate_node_chain(alloc);
            self.next = None;
        }
        self.deallocate_node_data(alloc);
        alloc.deallocate(self as *mut Self, 1);
    }

    unsafe fn deallocate_node_data(&mut self, alloc: &Allocator) {
        if let Some(data) = self.data {
            alloc.deallocate(data.as_ptr(), Self::ARRAY_LEN);
            self.data = None;
        }
    }

    /// Safety: only deallocate with the same allocator that allocated.
    /// Only call on the primary node
    pub unsafe fn deallocate(&mut self, alloc: &Allocator) {
        if let Some(next) = self.next_node_mut() {
            next.deallocate_node_chain(alloc)
        }
        self.deallocate_node_data(alloc);
    }

    unsafe fn get_data_ptr_mut(&mut self, idx: usize) -> *mut T {
        let mut node = self;
        let forward = idx / Self::ARRAY_LEN;
        for _i in 0..forward {
            node = node.next_node_mut().expect("next node not allocated");
        }
        let ptr = node.data.expect("accessing unallocated node data").as_ptr();
        ptr.add(idx % Self::ARRAY_LEN)
    }

    unsafe fn get_data_ptr(&self, idx: usize) -> *const T {
        let mut node = self;
        let forward = idx / Self::ARRAY_LEN;
        for _i in 0..forward {
            node = node.next_node().expect("next node not allocated");
        }
        let ptr = node.data.expect("accessing unallocated node data").as_ptr() as *const T;
        ptr.add(idx % Self::ARRAY_LEN)
    }

    pub unsafe fn get_data_unchecked_mut(&mut self, idx: usize) -> &mut T {
        &mut *self.get_data_ptr_mut(idx)
    }

    pub unsafe fn get_data_unchecked(&self, idx: usize) -> &T {
        &*self.get_data_ptr(idx)
    }

    pub unsafe fn write_data_unchecked(&mut self, idx: usize, val: T) {
        self.get_data_ptr_mut(idx).write(val)
    }
}

impl<T, N> Node<T, N> {
    fn next_node_mut(&mut self) -> Option<&mut Self> {
        self.next.as_mut().map(|node| unsafe { node.as_mut() })
    }

    fn next_node(&self) -> Option<&Self> {
        self.next.as_ref().map(|node| unsafe { node.as_ref() })
    }
}

impl<T, N> ops::Drop for Seque<T, N>
where
    N: Unsigned,
{
    fn drop(&mut self) {
        // Safety: all nodes below the first were allocated
        // all nodes will have their data deallocated
        unsafe {
            self.node.deallocate(&self.alloc);
        }
    }
}

pub struct Seque<T, N>
where
    N: Unsigned,
{
    length: usize,
    capacity: usize,
    node: Node<T, N>,
    alloc: Allocator,
}

impl<T, N> Seque<T, N>
where
    N: Unsigned,
{
    pub const NODE_ARRAY_LEN: usize = Node::<T, N>::ARRAY_LEN;

    pub fn node_array_len(&self) -> usize {
        Self::NODE_ARRAY_LEN
    }

    pub fn with_capacity_in(capacity: usize, alloc: Allocator) -> Self {
        let (node, capacity) = if capacity <= Self::NODE_ARRAY_LEN {
            (Node::with_data(&alloc), Self::NODE_ARRAY_LEN)
        } else {
            let mut parent = Node::with_data(&alloc);
            let capacity = {
                let mut i = 1 as usize;
                let cap = loop {
                    i += 1;
                    if Self::NODE_ARRAY_LEN * i >= capacity {
                        break Self::NODE_ARRAY_LEN * i;
                    }
                };
                parent.allocate_node_chain(i - 1, &alloc);
                cap
            };
            (parent, capacity)
        };
        Self {
            length: 0,
            capacity,
            node,
            alloc,
        }
    }

    pub fn push_back(&mut self, val: T) {
        if self.length >= self.capacity {
            let amount = self.length / self.capacity;
            self.node.allocate_node_chain(amount, &self.alloc);
            self.capacity += Self::NODE_ARRAY_LEN * amount;
        }
        unsafe { self.node.write_data_unchecked(self.length, val) }
        self.length += 1;
    }

    pub fn iter<'a>(&'a self) -> SequeIter<'a, T, N> {
        let ptr = NonNull::new(&self.node as *const _ as *mut _);
        SequeIter::new_iter_at(ptr, self.length)
    }

    pub fn iter_mut<'a>(&'a mut self) -> SequeIterMut<'a, T, N> {
        let ptr = NonNull::new(&self.node as *const _ as *mut _);
        SequeIterMut::new_iter_at(ptr, self.length)
    }
}

impl<T, N> ops::Index<usize> for Seque<T, N>
where
    N: Unsigned,
{
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.length {
            panic!(
                "index out of bounds: the len is {} but the idx is {}",
                self.length, index
            );
        }
        unsafe { self.node.get_data_unchecked(index) }
    }
}

impl<T, N> ops::IndexMut<usize> for Seque<T, N>
where
    N: Unsigned,
{
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.length {
            panic!(
                "index out of bounds: the len is {} but the idx is {}",
                self.length, index
            );
        }
        unsafe { self.node.get_data_unchecked_mut(index) }
    }
}

impl<T, N> a::AllocatorAwareContainer for Seque<T, N>
where
    N: Unsigned,
{
    fn allocator(&self) -> Allocator {
        self.alloc.clone()
    }
}

use core::slice::{self, Iter, IterMut};

pub struct SequeIter<'a, T, N> {
    current: Iter<'a, T>,
    next: Link<T, N>,
    len: usize,
}

impl<'a, T, N> SequeIter<'a, T, N> where N: Unsigned {
    pub(crate) fn new_iter_at(node: Link<T, N>, len: usize) -> Self {
        let mut s = Self {
            current: [].iter(),
            next: node,
            len,
        };
        s.update_next();
        s
    }

    fn update_next(&mut self) -> Option<()> {
        let new_curr = self.next?;
        let new_curr = unsafe { new_curr.as_ref() };
        self.next = new_curr.next;
        let len = if let Some(_) = self.next {
            N::USIZE
        } else {
            self.len % N::USIZE
        };
        // Safety: Immutable slice
        unsafe { self.current = slice::from_raw_parts(new_curr.get_data_ptr(0), len).iter() };
        Some(())
    }
}

impl<'a, T, N> Iterator for SequeIter<'a, T, N> where N: Unsigned {
    type Item = <Iter<'a, T> as Iterator>::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.current.next().or_else(|| {
            self.update_next()?;
            self.current.next()
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.len))
    }
}

pub struct SequeIterMut<'a, T, N> {
    current: IterMut<'a, T>,
    next: Link<T, N>,
    len: usize,
}

impl<'a, T, N> SequeIterMut<'a, T, N> where N: Unsigned {
    pub(crate) fn new_iter_at(node: Link<T, N>, len: usize) -> Self {
        let mut s = Self {
            current: [].iter_mut(),
            next: node,
            len,
        };
        s.update_next();
        s
    }

    fn update_next(&mut self) -> Option<()> {
        let mut new_curr = self.next?;
        let new_curr = unsafe { new_curr.as_mut() };
        self.next = new_curr.next;
        let len = if let Some(_) = self.next {
            N::USIZE
        } else {
            self.len % N::USIZE
        };
        // Safety: ???
        unsafe { self.current = slice::from_raw_parts_mut(new_curr.get_data_ptr_mut(0), len).iter_mut() };
        Some(())
    }
}

impl<'a, T, N> Iterator for SequeIterMut<'a, T, N> where N: Unsigned {
    type Item = <IterMut<'a, T> as Iterator>::Item;
    fn next(&mut self) -> Option<Self::Item> {
        self.current.next().or_else(|| {
            self.update_next()?;
            self.current.next()
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.len))
    }
}

impl<'a, T, N> IntoIterator for &'a Seque<T, N> where N: Unsigned {
    type IntoIter = SequeIter<'a, T, N>;
    type Item = <Self::IntoIter as Iterator>::Item;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T, N> IntoIterator for &'a mut Seque<T, N> where N: Unsigned {
    type IntoIter = SequeIterMut<'a, T, N>;
    type Item = <Self::IntoIter as Iterator>::Item;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}
