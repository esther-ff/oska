use crate::md::blocks::{Block, Parsed, Unparsed};
use core::num::NonZero;

#[derive(Debug)]
pub struct ListItem {
    number: Option<NonZero<usize>>,
    item: Box<Block<Unparsed>>,
}
