use crate::md::chars::{
    ASTERISK, BACKTICK, EQUALS, GREATER_THAN, HASH, LINE, NEWLINE, SPACE, UNDERSCORE,
};
use crate::walker::{StrRange, Walker};
use core::num::NonZero;
use core::str;

use super::chars::TILDE;

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
    info: Option<String>,
}

#[derive(Debug)]
enum Lang {
    None,
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

        let pred = |x: u8| (x == ASTERISK) | (x == LINE) | (x == UNDERSCORE);

        if pred(char) {
            match self.style_break(walker) {
                None => {}
                Some(sb) => return Some(sb),
            }
        }

        let block = match char {
            // Heading
            HASH => self.heading(walker),

            // Blockquote
            GREATER_THAN => self.blockquote(walker),

            // Fenced code
            BACKTICK => self.code::<BACKTICK>(walker),

            // Fenced code again!
            TILDE => self.code::<TILDE>(walker),

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
        let initial = walker.position();

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

    pub fn code<const CHAR: u8>(&mut self, walker: &mut Walker<'_>) -> Block {
        debug_assert!(
            CHAR == TILDE || CHAR == BACKTICK,
            "invalid char provided to the `code` function"
        );

        let amnt_of_backticks = walker.till_not(CHAR);

        if amnt_of_backticks < 2 {
            walker.retreat(amnt_of_backticks + 1);
            return self.paragraph(walker);
        }

        let pos = walker.position();
        let mut lang = Lang::None;
        let mut info = None;

        while let Some(char) = walker.next() {
            if char == CHAR {
                walker.set_position(pos);
                return self.paragraph(walker);
            };

            if char == NEWLINE {
                let range = StrRange::new(pos, walker.position() - 1);

                let mut split = range.resolve(walker.data()).split(",");

                lang = Lang::NotSupported(split.next().expect("always present").to_owned());

                info = split.next().map(|x| x.to_owned());

                break;
            }
        }

        let code_start = walker.position();
        let mut code_end = walker.position();
        while let Some(char) = walker.next() {
            if walker.is_next_char(CHAR) {
                let amnt_of = walker.till_not(CHAR);

                if amnt_of >= amnt_of_backticks {
                    code_end = walker.position() - amnt_of;
                    break;
                }
            }
        }

        let text = Some(StrRange::new(code_start, code_end));

        let meta = CodeMeta { lang, info };

        let code = Code {
            meta,
            text,
            id: self.get_new_id(),
        };

        Block::FencedCode(code)
    }

    pub fn heading(&mut self, walker: &mut Walker<'_>) -> Block {
        let level = if walker.is_next_char(HASH) {
            let temp = walker.till_not(HASH);

            if temp > 5 {
                walker.retreat(temp + 1);
                return self.paragraph(walker);
            };

            temp as u8 + 1
        } else {
            1
        };

        if !walker.is_next_char(SPACE) {
            walker.retreat(level as usize);
            return self.paragraph(walker);
        } else {
            walker.advance(1);
        }

        let text = walker.till(NEWLINE);

        let heading = Heading {
            level: HeadingLevel::new(level),
            text,
        };

        Block::Heading(heading)
    }

    pub fn style_break(&mut self, walker: &mut Walker<'_>) -> Option<Block> {
        let initial = walker.position();

        let pred = |x| (x == ASTERISK) | (x == LINE) | (x == UNDERSCORE);

        if walker.is_next_pred(pred) {
            walker.advance(1);

            if !walker.is_next_pred(pred) {
                walker.retreat(1);
                return None;
            } else {
                walker.advance(1);
            }
        } else {
            return None;
        };

        while let Some(char) = walker.next() {
            match char {
                ASTERISK | LINE | UNDERSCORE => {}

                NEWLINE => break,

                _ => {
                    walker.set_position(initial);
                    return None;
                }
            }
        }

        let brk = Block::StyleBreak(Break {
            id: self.get_new_id(),
        });

        Some(brk)
    }

    pub fn list(&mut self) -> Block {
        todo!()
    }
}

#[cfg(test)]
mod tests {

    use super::BlockParser;
    use crate::{block_parser::Block, walker::Walker};

    #[test]
    fn blockquote() {
        let md = concat!(
            ">>> This is a blockquote\n",
            ">>>> This is an another blockquote\n",
        )
        .as_bytes();

        let mut parser = BlockParser::new(());
        let mut walker = Walker::new(md);

        let val = parser
            .block(&mut walker)
            .expect("this block should be here");

        let inner = match val {
            Block::Blockquote(q) => *q.text.expect("field not present"),
            _ => panic!("block was not blockquote"),
        };

        match inner {
            Block::Paragraph(para) => {
                let text = para.text.unwrap();

                let resolved = text.resolve(md);

                assert!(resolved == "This is a blockquote\n");
            }

            _ => assert!(false, "block was not paragraph"),
        }

        let val = parser
            .block(&mut walker)
            .expect("this block should be here");

        let inner = match val {
            Block::Blockquote(q) => *q.text.expect("field not present"),
            _ => panic!("block was not blockquote"),
        };

        match inner {
            Block::Paragraph(para) => {
                let text = para.text.unwrap();

                let resolved = text.resolve(md);

                assert!(resolved == "This is an another blockquote\n");
            }

            _ => assert!(false, "block was not paragraph"),
        }
    }

    #[test]
    fn code() {
        let data = concat!("```rust\n", "#[no_std]\n", "```").as_bytes();

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new(());

        let block = match parser.block(&mut walker).expect("expected block") {
            Block::FencedCode(fc) => fc,

            _ => panic!("block was not fenced code"),
        };

        assert!(block.text.expect("text should be here").resolve(data) == "#[no_std]\n");
    }

    #[test]
    fn code_tilde() {
        let data = concat!("~~~rust\n", "#[no_std]\n", "~~~").as_bytes();

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new(());

        let block = match parser.block(&mut walker).expect("expected block") {
            Block::FencedCode(fc) => fc,

            _ => panic!("block was not fenced code"),
        };

        assert!(block.text.expect("text should be here").resolve(data) == "#[no_std]\n");
    }

    #[test]
    fn heading_simple() {
        let data = "###### une, grande, et indivisible".as_bytes();

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new(());

        let block = match parser.block(&mut walker).expect("expected block") {
            Block::Heading(h) => h,

            _ => panic!("block was not a heading"),
        };

        assert!(
            u8::from(block.level.0) == 6,
            "invalid level found, was supposed to be 6, is {}",
            block.level.0
        );
    }

    #[test]
    fn style_break_simple() {
        let data = concat!("___\n", "---\n", "***\n").as_bytes();

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new(());

        match parser.block(&mut walker).expect("expected block") {
            Block::StyleBreak(_) => {}

            _ => panic!("block was not style break"),
        };

        match parser.block(&mut walker).expect("expected block") {
            Block::StyleBreak(_) => {}

            _ => panic!("block was not style break"),
        };

        match parser.block(&mut walker).expect("expected block") {
            Block::StyleBreak(_) => {}

            _ => panic!("block was not style break"),
        };
    }
}
