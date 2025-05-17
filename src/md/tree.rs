use core::fmt::Debug;
use core::num::NonZero;

use super::ast::AstNode;

pub type NodeAst = Node<AstNode>;

#[cfg(target_pointer_width = "32")]
type Index = u16;

#[cfg(target_pointer_width = "64")]
type Index = u32;

#[derive(Eq, PartialEq, PartialOrd, Ord, Copy, Clone)]
#[repr(C)]
/// Allows to index into the `TreeArena`
/// internally contains an index to
/// the current chunk of memory used to allocate nodes
/// and the index of the node inside that chunk.
pub struct NodeId {
    node_index: Index,
    vec_index: NonZero<Index>,
}

impl NodeId {
    const HALF_PTR_WIDTH: usize = (size_of::<usize>() / 2);

    #[cfg(target_pointer_width = "32")]
    const MASK: usize = (2_usize.pow(Self::HALF_PTR_WIDTH as u16) << 16) - 1;

    #[cfg(target_pointer_width = "64")]
    const MASK: usize = (2_usize.pow(Self::HALF_PTR_WIDTH as u32) << 32) - 1;

    pub fn new(val: usize) -> Option<Self> {
        if val & Self::MASK == 0 {
            None
        } else {
            Some(NodeId::new_unchecked(val))
        }
    }

    pub fn new_unchecked(val: usize) -> Self {
        unsafe { core::mem::transmute::<usize, NodeId>(val) }
    }

    pub fn from_indexes(node_index: Index, vec_index: Index) -> Option<Self> {
        if vec_index == 0 {
            return None;
        };

        Self {
            vec_index: unsafe { NonZero::new_unchecked(vec_index) },
            node_index,
        }
        .into()
    }
}

impl Debug for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "NodeId {{ node_index: {}, vec_index: {} }}",
            self.node_index, self.vec_index
        )
    }
}

pub struct Node<T> {
    pub data: T,

    next: Option<NodeId>,
    prev: Option<NodeId>,

    child: Option<NodeId>,
}

impl<T: Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("data", &self.data)
            .field("next", &self.next)
            .field("prev", &self.prev)
            .finish()
    }
}

impl<T> Node<T> {
    pub fn new(data: T) -> Node<T> {
        Self {
            data,
            child: None,
            next: None,
            prev: None,
        }
    }

    pub fn previous_node(&mut self) -> Option<&mut NodeId> {
        self.prev.as_mut()
    }

    pub fn next_node(&mut self) -> Option<&mut NodeId> {
        self.next.as_mut()
    }

    pub fn child(&mut self) -> Option<&mut NodeId> {
        self.child.as_mut()
    }

    pub fn add_child<A>(&mut self, arena: &mut TreeArena<T>, child: A)
    where
        A: Into<Option<NodeId>>,
    {
        match self.child.into() {
            Some(id) => arena.get_mut(id),
            None => todo!(),
        }
    }
}

pub struct TreeArena<T> {
    tracker: Index,
    last_node: Option<NodeId>,
    first_node: Option<NodeId>,
    storage: Vec<Vec<Node<T>>>,
}

impl<T: Debug> Debug for TreeArena<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TreeArena")
            .field("tracker", &self.tracker)
            .field("last_node", &self.last_node)
            .field("first_node", &self.first_node)
            .finish_non_exhaustive()
    }
}

impl<T> Default for TreeArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
const AMOUNT_OF_BUCKETS: usize = 40;

impl<T> TreeArena<T> {
    const DEFAULT_BASE_SIZE: usize = (8 * 1024) / size_of::<T>();
    // const DEFAULT_BASE_SIZE_MINUS_ONE: usize = (8 * 1024) / size_of::<T>() - 1;

    pub fn new() -> TreeArena<T> {
        let mut storage = Vec::with_capacity(AMOUNT_OF_BUCKETS);
        // "canary" to ensure our indexes start from 1
        // so they are eligible for `NonZero`
        // therefore the size of `Option<NodeId>`
        // becomes the same as `NodeId`
        storage.push(Vec::new());

        // actual first "bucket"
        storage.push(Vec::with_capacity(Self::DEFAULT_BASE_SIZE));

        Self {
            last_node: None,
            first_node: None,
            tracker: 1,
            storage,
        }
    }

    pub fn get(&self, id: NodeId) -> Option<&Node<T>> {
        if NonZero::get(id.vec_index) > self.tracker
            || id.node_index > Self::DEFAULT_BASE_SIZE as u32
        {
            None
        } else {
            Some(unsafe { self.get_unchecked(id) })
        }
    }

    pub unsafe fn get_unchecked(&self, id: NodeId) -> &Node<T> {
        unsafe {
            self.storage
                .get_unchecked(NonZero::get(id.vec_index) as usize)
                .get_unchecked(id.node_index as usize)
        }
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node<T>> {
        if NonZero::get(id.vec_index) > self.tracker
            || id.node_index > Self::DEFAULT_BASE_SIZE as Index
        {
            None
        } else {
            Some(unsafe { self.get_unchecked_mut(id) })
        }
    }

    pub unsafe fn get_unchecked_mut(&mut self, id: NodeId) -> &mut Node<T> {
        unsafe {
            self.storage
                .get_unchecked_mut(NonZero::get(id.vec_index) as usize)
                .get_unchecked_mut(id.node_index as usize)
        }
    }

    #[inline]
    pub fn storage(&self) -> &[Node<T>] {
        &self.storage[self.tracker as usize]
    }

    #[inline]
    pub fn first_node(&self) -> Option<NodeId> {
        self.first_node
    }

    #[inline]
    pub fn last_node(&self) -> Option<NodeId> {
        self.last_node
    }

    #[inline]
    pub fn tracker(&self) -> Index {
        self.tracker
    }

    pub fn attach(&mut self, mut item: Node<T>) -> NodeId {
        item.prev = self.last_node;

        let ix = if self.storage_mut().len() == AMOUNT_OF_BUCKETS {
            self.slow_alloc(item, self.storage.len())
        } else {
            self.fast_alloc(item)
        };

        let new_node_id = NodeId::from_indexes(ix, self.tracker)
            .unwrap_or_else(|| unreachable!("vec indices should be larger than 0"));

        if let Some(last) = self.last_node {
            unsafe { self.get_unchecked_mut(last).next = Some(new_node_id) }
        };

        self.last_node = Some(new_node_id);
        new_node_id
    }

    fn slow_alloc(&mut self, item: Node<T>, cap: usize) -> Index {
        self.tracker += 1;

        let mut new_vec = Vec::with_capacity(cap);
        new_vec.push(item);

        self.storage.push(new_vec);

        0
    }

    fn fast_alloc(&mut self, item: Node<T>) -> Index {
        let len = self.storage_mut().len() as Index;
        self.storage_mut().push(item);

        len
    }

    #[inline]
    fn storage_mut(&mut self) -> &mut Vec<Node<T>> {
        &mut self.storage[self.tracker as usize]
    }
}
