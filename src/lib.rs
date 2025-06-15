// #![no_std]
#![warn(clippy::all)]
extern crate alloc;

pub(crate) mod lib {
    pub use alloc::boxed::Box;
    pub use alloc::string::String;
    pub use alloc::string::ToString;
    pub use alloc::vec::Vec;
}

mod ast;
mod tree;

pub mod block_parser;
pub mod walker;
