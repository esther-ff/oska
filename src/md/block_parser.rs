#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]

use crate::md::chars::{
    ASTERISK, BACKTICK, GREATER_THAN, HASH, LINE, NEWLINE, SPACE, TILDE, UNDERSCORE,
};

use super::blocks::style_break::style_break;
use super::blocks::{
    Block, Parsed, Unparsed, blockquote::blockquote, code::fenced::fenced_code,
    code::indented::indented_code, heading::heading, html_block::html_block,
    lists::bullet_list::bullet_list, lists::ordered_list::ordered_list, paragraph::paragraph,
    utils::is_bullet_list_marker,
};

use crate::walker::Walker;
use core::marker::PhantomData;
use core::str;

use super::chars::LESSER_THAN;
static BLOCK_VEC_PREALLOCATION: usize = 64;

/// Structure that represents a Markdown document
/// it has 2 states: `Unparsed` and `Parsed`
/// those relate to the state of inline elements
/// inside blocks inside of this document
#[derive(Debug)]
pub struct Document<State> {
    blocks: Option<Vec<Block<State>>>,
}

impl Document<Unparsed> {
    pub fn new() -> Self {
        Self { blocks: None }
    }

    pub fn add(&mut self, val: Block<Unparsed>) {
        match self.blocks {
            None => {
                let mut vec = Vec::with_capacity(BLOCK_VEC_PREALLOCATION);

                vec.push(val);

                self.blocks.replace(vec);
            }

            Some(ref mut vec) => vec.push(val),
        }
    }

    pub fn parse_inlines(&mut self) -> ! {
        todo!()
    }
}

impl Default for Document<Unparsed> {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait responsible for parsing text into Block elements
pub trait BlockParser {
    fn block(&mut self, walker: &mut Walker) -> Block<Unparsed>;
    fn get_new_id(&mut self) -> usize;
    fn document(self, walker: &mut Walker) -> Document<Unparsed>;
}

/// Structure reponsible for parsing a document
/// into it's block structure
/// which will allow for parallel processing of
/// inline content
pub struct DefaultParser {
    col: Document<Unparsed>,
    id: usize,
}

impl DefaultParser {
    /// Creates a new `BlockParser`
    pub fn new() -> Self {
        Self {
            col: Document::default(),
            id: 0,
        }
    }
}

impl Default for DefaultParser {
    fn default() -> Self {
        Self::new()
    }
}

impl BlockParser for DefaultParser {
    /// Populates an document with `Block`s
    /// without parsed inline contents
    fn document(mut self, walker: &mut Walker<'_>) -> Document<Unparsed> {
        loop {
            let val = match self.block(walker) {
                Block::Eof => break,
                val => val,
            };

            self.col.add(val);
        }

        self.col
    }

    /// Generates a new id
    fn get_new_id(&mut self) -> usize {
        let id = self.id;

        self.id += 1;

        id
    }

    /// Parses some block into a "unparsed state"
    /// meaning the block has text contents that are yet to be parsed
    /// into inlines via a `InlineParser`.
    fn block(&mut self, walker: &mut Walker<'_>) -> Block<Unparsed> {
        let Some(char) = walker.next() else {
            return Block::Eof;
        };

        let pred = |x: u8| (x == ASTERISK) | (x == LINE) | (x == UNDERSCORE);

        if pred(char) {
            match style_break(self, walker) {
                None => {}
                Some(sb) => return sb,
            }
        }

        match char {
            // Heading
            HASH => match heading(self, walker) {
                None => paragraph(self, walker),
                Some(val) => val,
            },

            // Blockquote
            GREATER_THAN => blockquote(self, walker),
            // Fenced code
            BACKTICK => fenced_code::<BACKTICK>(self, walker),

            // Fenced code again!
            TILDE => fenced_code::<TILDE>(self, walker),

            // Indented code
            SPACE => match indented_code(self, walker) {
                None => {
                    // walker.retreat(1);
                    self.block(walker)
                }

                Some(block) => block,
            },

            char if char.is_ascii_digit() => {
                let start = str::from_utf8(&[char])
                    .expect("should always be correct utf-8")
                    .parse::<usize>()
                    .expect("should be a correct number in string form");

                ordered_list(self, start, walker)
            }

            char if is_bullet_list_marker(char) => bullet_list(self, char, walker),

            LESSER_THAN => html_block(self, walker),

            NEWLINE => {
                walker.advance(1);
                self.block(walker)
            }

            _ => {
                // dbg!(core::str::from_utf8(&[any]));
                walker.retreat(1);

                paragraph(self, walker)
            }
        }
    }
}
