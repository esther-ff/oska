use core::cell::{Cell, RefCell};
use core::fmt::Debug;

/// Arena allocator
///
/// Internally uses a `RefCell`
/// Used to store `Node`s
#[derive(Debug)]
pub(crate) struct Arena<'a, T> {
    inner: RefCell<Chunks<T>>,
    prev: Cell<Option<&'a T>>,
    first: Cell<Option<&'a T>>,
}

#[derive(Debug)]
struct Chunks<T> {
    cur: Vec<T>,
    rst: Vec<Vec<T>>,
}

impl<T> Chunks<T> {
    const START_SIZE: usize = 1024 * 4;

    fn new() -> Self {
        assert_ne!(size_of::<T>(), 0);

        Self {
            cur: Vec::with_capacity(Self::START_SIZE / size_of::<T>()),
            rst: Vec::new(),
        }
    }
}

impl<'a, T> Arena<'a, T> {
    pub(crate) fn new() -> Arena<'a, T> {
        Self {
            inner: RefCell::new(Chunks::new()),
            prev: Cell::new(None),
            first: Cell::new(None),
        }
    }

    pub(crate) fn previous(&self) -> Option<&'a T> {
        self.prev.get()
    }

    pub(crate) fn first(&self) -> Option<&'a T> {
        self.first.get()
    }

    pub(crate) fn alloc(&self, item: T) -> &T {
        let mut chunk = self.inner.borrow_mut();
        let cur_len = chunk.cur.len();

        let reff = if cur_len < chunk.cur.capacity() {
            chunk.cur.push(item);

            unsafe { &*chunk.cur.as_ptr().add(cur_len) }
        } else {
            let mut new_chunk = Vec::with_capacity(chunk.cur.capacity());
            new_chunk.push(item);
            let old_chunk = core::mem::replace(&mut chunk.cur, new_chunk);
            chunk.rst.push(old_chunk);
            unsafe { &*chunk.cur.as_ptr() }
        };

        self.prev.replace(Some(reff));

        if self.first.get().is_none() {
            self.first.replace(Some(reff));
        }

        reff
    }
}

impl<'a, T> Arena<'a, Node<'a, T>> {
    pub(crate) fn to_list(&'a self, val: T) -> &'a Node<'a, T> {
        let old = self.previous();
        let new = self.alloc(Node::new(val, None, old));

        if let Some(node) = old {
            node.next.set(Some(new));
        }

        new
    }
}

/// A node belonging to a linked list
pub(crate) struct Node<'a, T> {
    pub val: RefCell<T>,
    pub prev: Cell<Option<&'a Node<'a, T>>>,
    pub next: Cell<Option<&'a Node<'a, T>>>,
}

impl<'a, T> Node<'a, T> {
    pub(crate) fn new<A, B>(val: T, next: A, prev: B) -> Self
    where
        A: Into<Option<&'a Self>>,
        B: Into<Option<&'a Self>>,
    {
        Node {
            val: RefCell::new(val),
            next: Cell::new(next.into()),
            prev: Cell::new(prev.into()),
        }
    }
}

impl<T: Debug> Debug for Node<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("val", &self.val)
            .field("prev", &self.prev.get())
            .field("next", &self.prev.get())
            .finish()
    }
}

pub type NodeRef<'a, T> = &'a Node<'a, T>;
