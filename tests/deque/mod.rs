use core::{marker::PhantomData, ops, ptr::NonNull};
use typenum::Unsigned;
use yaap::a::{self, Allocator};

type Data<T> = a::Ptr<T>;
type Link<T, N> = Data<Node<T, N>>;

struct Node<T, N> {
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
