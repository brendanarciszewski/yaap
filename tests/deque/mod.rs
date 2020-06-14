use core::{cell::RefCell, ops, ptr::NonNull};
use std::rc::Rc;
use yaap::a::{self, Allocator};

type Data<T> = a::Ptr<T>;
type Link<T> = Data<Node<T>>;

pub static NODE_ARRAY_LEN: usize = 16;

struct Node<T> {
    data: Data<T>,
    next: Link<T>,
}

impl<T> Node<T> {
    // pub fn new() -> Self {
    //     DequeNode {
    //         data: None,
    //         next: None,
    //     }
    // }

    #[inline(always)]
    fn allocate_node_data(alloc: &RefCell<dyn Allocator>) -> Data<T> {
        // SAFETY: data is never accessed before first writing a valid value
        unsafe { a::allocate::<T>(alloc, NODE_ARRAY_LEN).map(NonNull::cast) }
    }

    pub fn with_data(alloc: &RefCell<dyn Allocator>) -> Self {
        Node {
            data: Node::allocate_node_data(alloc),
            next: None,
        }
    }

    pub fn allocate_next_node(&mut self, alloc: &RefCell<dyn Allocator>) {
        if self.next.is_some() {
            panic!("Replacing existing node");
        }

        self.next = unsafe {
            a::allocate::<Node<T>>(alloc, 1).map(|mut node| {
                node.as_mut().as_mut_ptr().write(Node {
                    data: Node::allocate_node_data(alloc),
                    next: None,
                });
                node.cast()
            })
        };

        if self.next.is_none() {
            panic!("allocation of node failed");
        }
    }

    pub fn next_node_mut(&mut self) -> Option<&mut Self> {
        self.next.as_mut().map(|node| unsafe { node.as_mut() })
    }

    #[inline]
    unsafe fn get_ptr_mut(&mut self, idx: usize) -> *mut T {
        let mut node = self;
        let forward = idx / NODE_ARRAY_LEN;
        for _i in 0..forward {
            node = node
                .next
                .as_mut()
                .expect("Accessing uninitialized block")
                .as_mut();
        }
        let ptr = node.data.expect("Accessing uninitialized memory").as_mut() as *mut T;
        ptr.add(idx % NODE_ARRAY_LEN)
    }

    #[inline]
    unsafe fn get_ptr(&self, idx: usize) -> *const T {
        let mut node = self;
        let forward = idx / NODE_ARRAY_LEN;
        for _i in 0..forward {
            node = node
                .next
                .as_ref()
                .expect("Accessing uninitialized block")
                .as_ref();
        }
        let ptr = node.data.expect("Accessing uninitialized memory").as_mut() as *mut T;
        ptr.add(idx % NODE_ARRAY_LEN)
    }

    pub unsafe fn get_unchecked_mut(&mut self, idx: usize) -> &mut T {
        &mut *self.get_ptr_mut(idx)
    }

    pub unsafe fn get_unchecked(&self, idx: usize) -> &T {
        &*self.get_ptr(idx)
    }

    pub unsafe fn write_unchecked(&mut self, idx: usize, val: T) {
        self.get_ptr_mut(idx).write(val)
    }
}

impl<T> ops::Drop for Seque<T> {
    fn drop(&mut self) {
        //for _v in self.into_iter() {}
    }
}
pub struct Seque<T> {
    length: usize,
    capacity: usize,
    node: Node<T>,
    alloc: Rc<RefCell<dyn Allocator>>,
}

impl<T> Seque<T> {
    pub fn with_capacity_in(capacity: usize, alloc: Rc<RefCell<dyn Allocator>>) -> Self {
        let (node, capacity) = if capacity <= NODE_ARRAY_LEN {
            (Node::with_data(alloc.as_ref()), NODE_ARRAY_LEN)
        } else {
            let mut parent = Node::with_data(alloc.as_ref());
            let mut i = 1 as usize;
            let capacity = loop {
                i += 1;
                if NODE_ARRAY_LEN * i >= capacity {
                    break NODE_ARRAY_LEN * i;
                }
            };
            let mut node = &mut parent;
            while i > 1 {
                node.allocate_next_node(alloc.as_ref());
                i -= 1;
                node = node.next_node_mut().expect("Just allocated");
            }
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
        while self.length >= self.capacity {
            self.node.allocate_next_node(self.alloc.as_ref());
            self.capacity += NODE_ARRAY_LEN;
        }
        unsafe { self.node.write_unchecked(self.length, val) }
        self.length += 1;
    }
}

impl<T> ops::Index<usize> for Seque<T> {
    type Output = T;
    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.length {
            panic!(
                "index out of bounds: the len is {} but the idx is {}",
                self.length, index
            );
        }
        unsafe { self.node.get_unchecked(index) }
    }
}

impl<T> ops::IndexMut<usize> for Seque<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.length {
            panic!(
                "index out of bounds: the len is {} but the idx is {}",
                self.length, index
            );
        }
        unsafe { self.node.get_unchecked_mut(index) }
    }
}

impl<T> a::AllocatorAwareContainer for Seque<T> {
    fn allocator(&self) -> Rc<RefCell<dyn Allocator>> {
        self.alloc.clone()
    }
}
