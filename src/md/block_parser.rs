use crate::md::chars::{
    ASTERISK, BACKTICK, EQUALS, GREATER_THAN, HASH, LINE, NEWLINE, SPACE, UNDERSCORE,
};
use crate::walker::{StrRange, Walker};
use core::num::NonZero;
use core::str;

static BLOCK_VEC_PREALLOCATION: usize = 64;

#[derive(Debug)]
pub(crate) struct Paragraph {
    text: Option<StrRange>,
    id: usize,
}

#[derive(Debug)]
pub(crate) struct BlkQt {
    level: BlkQtLevel,
    text: Option<Box<Block>>,
    id: usize,
}

#[derive(Debug)]
pub(crate) struct BlkQtLevel(usize);

impl BlkQtLevel {
    fn new(level: usize) -> Self {
        Self(level)
    }
}

// TODO: Lists...
#[derive(Debug)]
pub(crate) struct List;

#[derive(Debug)]
pub(crate) struct Code {
    meta: CodeMeta,
    text: Option<StrRange>,
    id: usize,
}

#[derive(Debug)]
pub(crate) struct CodeMeta {
    lang: Lang,
    info: String,
    id: usize,
}

#[derive(Debug)]
enum Lang {
    Rust,
    NotSupported(String),
}

#[derive(Debug)]
pub(crate) struct Heading {
    level: HeadingLevel,
    text: Option<StrRange>,
}

#[derive(Debug)]
pub(crate) struct HeadingLevel(NonZero<u8>);

#[derive(Debug)]
pub(crate) struct Break {
    id: usize,
}

impl HeadingLevel {
    fn new(level: u8) -> Self {
        debug_assert!(level > 0);

        let nonzero = unsafe { NonZero::new_unchecked(level) };

        Self(nonzero)
    }
}

#[derive(Debug)]
pub enum Block {
    Paragraph(Paragraph),
    Blockquote(BlkQt),
    List(List),
    FencedCode(Code),
    Heading(Heading),
    StyleBreak(Break),
}

impl Block {
    pub fn str_range<F>(&mut self, func: F)
    where
        F: FnOnce(&mut StrRange),
    {
        match self {
            Self::Paragraph(para) => func(para.text.as_mut().expect("should be here")),
            Self::Blockquote(qt) => {
                Block::str_range(qt.text.as_mut().expect("should be here"), func)
            }
            Self::List(_) => todo!("str_range: list"),
            Self::FencedCode(code) => func(code.text.as_mut().expect("should be here")),
            Self::Heading(heading) => func(heading.text.as_mut().expect("should be here")),
            Self::StyleBreak(_) => {}
        }
    }
}

pub(crate) struct BlockParser {
    col: Vec<Block>,
    id: usize,
}

impl BlockParser {
    pub fn new(_data: ()) -> Self {
        Self {
            col: Vec::with_capacity(64),
            id: 0,
        }
    }

    pub fn get_new_id(&mut self) -> usize {
        let id = self.id;

        self.id += 1;

        id
    }

    pub fn block(&mut self, walker: &mut Walker<'_>) -> Option<Block> {
        let char = match walker.next() {
            Some(c) => c,
            None => return None,
        };

        let block = match char {
            // Blockquote
            GREATER_THAN => self.blockquote(walker),

            _ => self.paragraph(walker),
        };

        Some(block)
    }

    pub fn paragraph(&mut self, walker: &mut Walker<'_>) -> Block {
        let initial = walker.position();

        while let Some(char) = walker.next() {
            match char {
                ch if (ch == NEWLINE) && walker.is_next_char(NEWLINE) => break,

                GREATER_THAN => {
                    walker.retreat(1);
                    break;
                }

                _ => {}
            };
        }

        let range = StrRange::new(initial, walker.position());

        let para = Paragraph {
            text: Some(range),
            id: self.get_new_id(),
        };

        Block::Paragraph(para)
    }

    pub fn blockquote(&mut self, walker: &mut Walker<'_>) -> Block {
        let id = self.get_new_id();
        let level = walker.till_not(GREATER_THAN);
        let initial = walker.position() + walker.is_next_char(SPACE) as usize;

        while let Some(char) = walker.next() {
            match char {
                NEWLINE => {
                    if walker.is_next_char(NEWLINE) {
                        walker.advance(1);
                        break;
                    }
                }

                GREATER_THAN => {
                    let amnt_of = walker.till_not(GREATER_THAN);

                    if amnt_of != level {
                        walker.retreat(amnt_of + 1);
                        break;
                    }
                }

                _ => {}
            }
        }

        let bytes = walker
            .get(initial, walker.position())
            .expect("this access should be in range");

        let (start, end) = bytes.get();

        let mut new_walker = Walker::new(&walker.data()[start..end]);
        let inner = match self.block(&mut new_walker) {
            None => None,

            Some(mut val) => {
                val.str_range(|range| {
                    range.adjust(|(start, end)| {
                        *start += initial;
                        *end += initial;
                    })
                });

                Some(Box::new(val))
            }
        };

        let blk = BlkQt {
            level: BlkQtLevel(level),
            text: inner,
            id,
        };

        Block::Blockquote(blk)
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

#[cfg(test)]
mod tests {

    use crate::walker::Walker;

    use super::BlockParser;

    #[test]
    fn blockquote() {
        let md = concat!(
            ">>> This is a blockquote\n",
            ">>>> This is another blockquote\n",
        );

        let mut parser = BlockParser::new(());
        let mut walker = Walker::new(md.as_bytes());

        let val = parser.block(&mut walker).unwrap();
        dbg!(val);
        let val = parser.block(&mut walker);
        dbg!(val);
    }
}
