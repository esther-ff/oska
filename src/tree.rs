use super::ast::AstNode;
use crate::lib::Vec;
use core::fmt::Debug;
use core::num::NonZero;

extern crate std;

pub type NodeAst = Node<AstNode>;

#[cfg(target_pointer_width = "32")]
type Index = u16;

#[cfg(target_pointer_width = "64")]
type Index = u32;

/// Allows to index into the `TreeArena`
/// internally contains an index to
/// the current chunk of memory used to allocate nodes
/// and the index of the node inside that chunk.
#[derive(Eq, PartialEq, PartialOrd, Ord, Copy, Clone)]
#[repr(C)]
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
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
    child: Option<NodeId>,
}

impl<T: Debug> Debug for Node<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Node {{ data: {:?}, next: {:?}, child: {:?}}}",
            &self.data, self.next, self.child
        )
    }
}

impl<T> Node<T> {
    pub fn new(data: T) -> Node<T> {
        Self {
            data,
            child: None,
            next: None,
        }
    }

    pub fn next_node(&mut self) -> Option<&mut NodeId> {
        self.next.as_mut()
    }

    pub fn child(&mut self) -> Option<&mut NodeId> {
        self.child.as_mut()
    }

    pub fn add_child(&mut self, arena: &mut TreeArena<T>, child: T) {
        match self.child {
            Some(id) => {
                let child_id = arena.isolated_node(child);

                unsafe {
                    arena.private_add_child_to_parent(id, child_id);
                }
            }
            None => self.child = Some(arena.isolated_node(child)),
        }
    }
}

pub struct TreeArena<T> {
    /// Index of currently used `Vec`.
    tracker: Index,

    /// Points to the current `Node` being processed.
    cursor: Option<NodeId>,

    /// Stores the `NodeId`s pointing to nodes
    /// as a path to the current node
    right_edge: Vec<NodeId>,

    /// Stores `Node`s inside
    /// the tree structure.
    storage: Vec<Vec<Node<T>>>,
}

impl<T: Debug> Debug for TreeArena<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        use crate::lib::String;
        fn visit_disk<T: Debug>(
            tree: &TreeArena<T>,
            id: NodeId,
            f: &mut core::fmt::Formatter<'_>,
            ident: String,
        ) -> core::fmt::Result {
            let disk = tree.get(id).unwrap();

            let mut new_ident = String::from(ident.as_str());
            new_ident.push_str("  ");

            match (disk.next, disk.child) {
                (Some(next), Some(child)) => {
                    writeln!(
                        f,
                        "{ident}Node: (child: {:?}, next: {:?}, val: {:?})",
                        child, next, disk.data
                    )?;

                    visit_disk(tree, child, f, new_ident.clone())?;

                    visit_disk(tree, next, f, new_ident)?;
                }
                (Some(next), None) => {
                    writeln!(f, "{ident}Next: (next: {:?}, val: {:?})\n", next, disk.data)?;

                    visit_disk(tree, next, f, new_ident)?;
                }
                (_, Some(child)) => {
                    writeln!(
                        f,
                        "{ident}Child: (child: {:?}, val: {:?})\n",
                        child, disk.data
                    )?;

                    visit_disk(tree, child, f, new_ident)?;
                }

                _ => {
                    write!(f, "{ident}Leaf: (val: {:?})\n\n", disk.data)?;
                }
            }

            Ok(())
        }

        fn debug_tree<T: Debug>(
            tree: &TreeArena<T>,
            f: &mut core::fmt::Formatter<'_>,
        ) -> core::fmt::Result {
            match tree.storage.first() {
                None => Ok(()),

                Some(_) => visit_disk(
                    tree,
                    NodeId {
                        node_index: 0,
                        vec_index: NonZero::new(1).unwrap(),
                    },
                    f,
                    String::from("  "),
                ),
            }
        }

        debug_tree(self, f)
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
            tracker: 1,
            cursor: None,
            right_edge: Vec::with_capacity(32),
            storage,
        }
    }

    pub fn get(&self, id: NodeId) -> Option<&Node<T>> {
        let ix: usize = NonZero::get(id.vec_index) as usize;
        let vec_ix: Index = NonZero::get(id.vec_index);

        if vec_ix > self.tracker || id.node_index as usize >= self.storage[ix].len() {
            return None;
        }

        Some(unsafe { self.get_unchecked(id) })
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
    pub fn tracker(&self) -> Index {
        self.tracker
    }

    pub fn attach_node(&mut self, item: T) -> NodeId {
        let ix = self.isolated_node(item);
        dbg!(&self.right_edge);
        if let Some(cur) = self.cursor {
            println!("meow");
            self.get_mut(cur).unwrap().next = Some(ix)
        } else if let Some(&parent) = self.right_edge.last() {
            self.get_mut(parent).unwrap().child = Some(ix)
        }

        self.cursor = Some(ix);

        ix
    }

    pub fn go_up(&mut self) -> Option<NodeId> {
        let ix = self.right_edge.pop()?;
        self.cursor = Some(ix);

        Some(ix)
    }

    pub fn go_down(&mut self) -> NodeId {
        let ix = self.cursor.unwrap();
        self.right_edge.push(ix);

        self.cursor = self.get(ix).and_then(|x| x.child);
        ix
    }

    pub fn attach_child(&mut self, data: T) -> NodeId {
        let ix = self.isolated_node(data);

        match self.cursor {
            None => {}

            Some(prev_cur) => unsafe { self.get_unchecked_mut(prev_cur).child = Some(ix) },
        }

        self.cursor = Some(ix);
        ix
    }

    pub fn attach_next(&mut self, data: T) -> NodeId {
        let ix = self.isolated_node(data);

        match self.cursor {
            None => {}

            Some(prev_cur) => unsafe { self.get_unchecked_mut(prev_cur).next = Some(ix) },
        }

        self.cursor = Some(ix);
        ix
    }

    fn isolated_node(&mut self, data: T) -> NodeId {
        let item = Node {
            data,
            next: None,
            child: None,
        };

        let ix = if self.storage_mut().len() == AMOUNT_OF_BUCKETS {
            self.slow_alloc(item, self.storage.len())
        } else {
            self.fast_alloc(item)
        };

        NodeId::from_indexes(ix, self.tracker)
            .unwrap_or_else(|| unreachable!("vec indices should be larger than 0"))
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

    #[inline]
    unsafe fn private_add_child_to_parent(&mut self, parent: NodeId, child: NodeId) {
        let patient = unsafe { self.get_unchecked_mut(parent) };

        match patient.child {
            None => {
                patient.child = Some(child);
            }
            Some(actual_child) => unsafe {
                self.private_add_child_to_parent(actual_child, child);
            },
        }
    }
}

impl TreeArena<AstNode> {
    pub fn preorder_visit<V>(&self, visitor: &V)
    where
        V: Visitor,
    {
        self.inner_preorder(
            Some(NodeId {
                node_index: 0,
                vec_index: NonZero::new(1).expect("infallible"),
            }),
            visitor,
        );
    }

    fn inner_preorder<V>(&self, node: Option<NodeId>, visitor: &V)
    where
        V: Visitor,
    {
        let Some(id) = node else { return };

        let Some(target) = self.get(id) else {
            return;
        };

        visitor.visit_node(&target.data);

        self.inner_preorder(target.child, visitor);
        self.inner_preorder(target.next, visitor);
    }

    pub fn preorder_visit_mut<V>(&mut self, visitor: &mut V)
    where
        V: MutVisitor,
    {
        self.inner_preorder_mut(
            Some(NodeId {
                node_index: 0,
                vec_index: NonZero::new(1).expect("infallible"),
            }),
            visitor,
        );
    }

    fn inner_preorder_mut<V>(&mut self, node: Option<NodeId>, visitor: &mut V)
    where
        V: MutVisitor,
    {
        let Some(id) = node else { return };

        if let Some(target) = self.get_mut(id) {
            visitor.visit_node_mut(&mut target.data);
        }

        self.inner_preorder_mut(self.get(id).and_then(|x| x.child), visitor);
        self.inner_preorder_mut(self.get(id).and_then(|x| x.child), visitor);
    }
}

pub trait Visitor {
    fn visit_node(&self, value: &AstNode);
}

pub trait MutVisitor {
    fn visit_node_mut(&mut self, value: &mut AstNode);
}
