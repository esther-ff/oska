#![no_std]

extern crate alloc;

pub(crate) mod lib {
    pub use alloc::boxed::Box;
    pub use alloc::string::String;
    pub use alloc::vec::Vec;
}

mod md;

pub use md::block_parser;
pub use md::inline_parser;
pub use md::walker;

mod tests;
