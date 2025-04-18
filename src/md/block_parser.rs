use crate::md::chars::{ASTERISK, BACKTICK, EQUALS, GREATER_THAN, HASH, LINE, SPACE, UNDERSCORE};
use crate::walker::Walker;
use core::str;

static BLOCK_VEC_PREALLOCATION: usize = 64;

pub(crate) struct Block<'b> {
    data: Option<&'b str>,
    blk_type: BlockType<'b>,
    id: usize,
}

enum BlockType<'a> {
    Paragraph,
    Blockquote,
    List,
    FencedCode(&'a str),
    Heading(usize),
    StyleBreak,
}

pub(crate) struct BlockParser<'p> {
    data: Walker<'p>,
    col: Vec<Block<'p>>,
    id: usize,
}

impl<'p> BlockParser<'p> {
    pub fn new(data: &'p [u8]) -> Self {
        Self {
            data: Walker::new(data),
            col: Vec::with_capacity(64),
            id: 0,
        }
    }

    pub fn get_new_id(&mut self) -> usize {
        let id = self.id;

        self.id += 1;

        id
    }

    pub fn block(&mut self) -> Option<Block> {
        let char = match self.data.next() {
            Some(c) => c,
            None => return None,
        };

        let block = match char {
            // Blockquote
            GREATER_THAN => self.blockquote(),
        };

        Some(block)
    }

    pub fn paragraph(&mut self) -> Block {
        todo!()
    }

    pub fn blockquote(&mut self) -> Block {
        while let Some(char) = self.data.next() {
            match char {
                NEWLINE => {
                    if self.data.is_next_char(GREATER_THAN) || self.data.is_next_char(NEWLINE) {}
                }

                _ => {}
            }
        }
    }

    pub fn code(&mut self) -> Block {
        todo!()
    }

    pub fn list(&mut self) -> Block {
        todo!()
    }

    pub fn style_break(&mut self) -> Block {
        todo!()
    }
}
