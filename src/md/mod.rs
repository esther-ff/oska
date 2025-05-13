mod arena;
mod ast;
mod chars;
mod html_constants;

pub mod inline_parser;
pub mod inlines;

pub mod block_parser;
pub mod blocks;

pub use block_parser::{BlockParser, DefaultParser, Document};
pub use blocks::Block;

pub mod walker;
pub use walker::Walker;

pub struct CompleteParser;
