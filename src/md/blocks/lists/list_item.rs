use crate::md::blocks::{Block, Parsed, Unparsed};
use core::num::NonZero;

#[derive(Debug)]
pub struct ListItem<State> {
    number: Option<NonZero<usize>>,
    item: Box<Block<State>>,
}

impl ListItem<Unparsed> {
    pub fn new<A: Into<Option<NonZero<usize>>>>(num: A, item: Block<Unparsed>) -> Self {
        Self {
            number: num.into(),
            item: Box::new(item),
        }
    }
}

impl<State> ListItem<State> {
    pub fn inner(&mut self) -> &mut Block<State> {
        &mut *self.item
    }

    pub fn number(&self) -> Option<NonZero<usize>> {
        self.number
    }
}
