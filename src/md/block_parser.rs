#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]
use crate::md::chars::{
    ASTERISK, BACKTICK, DOT, EQUALS, GREATER_THAN, HASH, LINE, NEWLINE, RIGHT_PAREN, SPACE, TILDE,
    UNDERSCORE,
};
use crate::walker::{StrRange, Walker};
use core::num::NonZero;
use core::str;

static BLOCK_VEC_PREALLOCATION: usize = 64;

#[derive(Debug)]
pub struct Paragraph {
    text: Option<StrRange>,
    id: usize,
}

#[derive(Debug)]
pub struct BlkQt {
    level: BlkQtLevel,
    text: Option<Box<Block>>,
    id: usize,
}

#[derive(Debug)]
pub struct BlkQtLevel(usize);

impl BlkQtLevel {
    fn new(level: usize) -> Self {
        Self(level)
    }
}

// TODO: Lists...
#[derive(Debug)]
pub enum List {
    Ordered(OrderedList),
    Bullet(BulletList),
}

#[derive(Debug)]
pub struct OrderedList {
    tight: bool,
    start_number: usize,
    items: Vec<ListItem>,
    id: usize,
}

struct OListConstructor {
    items: Vec<ListItem>,
    num: usize,
    cache: usize,
}

impl OListConstructor {
    pub fn new(num: usize) -> Self {
        Self {
            items: Vec::new(),
            num,
            cache: num,
        }
    }

    pub fn push_item(&mut self, item: Block) {
        self.num += 1;
        // Safety:
        //
        // Valid lists start from minimally the number 0
        // and we add 1 at the start
        // which means the number at least will be 1
        // so it qualifies for `NonZero<usize>`
        let number: Option<NonZero<usize>> = unsafe { NonZero::new_unchecked(self.num) }.into();
        let list_item = ListItem {
            item: Box::new(item),
            number,
        };

        self.items.push(list_item);
    }

    pub fn finish(self, id: usize, tight: bool) -> Block {
        Block::make_ordered_list(self.cache, self.items, tight, id)
    }
}

#[derive(Debug)]
pub struct BulletList {
    tight: bool,
    items: Vec<ListItem>,
    id: usize,
}

#[derive(Debug)]
pub struct ListItem {
    number: Option<NonZero<usize>>,
    item: Box<Block>,
}

#[derive(Debug)]
pub struct Code {
    meta: CodeMeta,
    text: Option<StrRange>,
    id: usize,
}

#[derive(Debug)]
pub struct IndentCode {
    indents: Box<[StrRange]>,
    id: usize,
}

#[derive(Debug)]
pub struct CodeMeta {
    lang: Lang,
    info: Option<String>,
}

#[derive(Debug)]
pub enum Lang {
    None,
    Rust,
    NotSupported(Box<str>),
}

impl Lang {
    pub fn is_useless(&self) -> bool {
        matches!(self, Self::None | Self::NotSupported(_))
    }

    pub fn recognize(name: &str) -> Lang {
        match name {
            "rust" => Lang::Rust,

            "" => Lang::None,

            unknown => Lang::NotSupported(unknown.to_string().into_boxed_str()),
        }
    }
}

#[derive(Debug)]
pub struct Heading {
    level: HeadingLevel,
    text: Option<StrRange>,
    id: usize,
}

#[derive(Debug)]
pub struct HeadingLevel(NonZero<u8>);

#[derive(Debug)]
pub struct Break {
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
    IndentedCode(IndentCode),
    Heading(Heading),
    StyleBreak(Break),
    Eof,
}

impl Block {
    pub fn str_range<F>(&mut self, func: F)
    where
        F: FnOnce(&mut StrRange),
    {
        match self {
            Self::Paragraph(para) => func(para.text.as_mut().expect("should be here")),
            Self::Blockquote(qt) => {
                Block::str_range(qt.text.as_mut().expect("should be here"), func);
            }
            Self::List(_) => todo!("str_range: list"),
            Self::FencedCode(code) => func(code.text.as_mut().expect("should be here")),
            Self::Heading(heading) => func(heading.text.as_mut().expect("should be here")),
            Self::Eof => panic!("temporary: panicked due to running `str_range` on a `Block::Eof`"),

            _ => {}
        }
    }

    pub fn adjust_range(&mut self, to_start: usize, to_end: usize) {
        self.str_range(|r| {
            r.adjust(|(start, end)| {
                *start += to_start;
                *end += to_end;
            });
        });
    }

    #[inline]
    pub fn make_paragraph(range: impl Into<Option<StrRange>>, id: usize) -> Block {
        Block::Paragraph(Paragraph {
            text: range.into(),
            id,
        })
    }

    #[inline]
    pub fn make_blockquote(range: impl Into<Option<Block>>, id: usize, level: usize) -> Block {
        Block::Blockquote(BlkQt {
            level: BlkQtLevel::new(level),
            text: range.into().map(Box::new),
            id,
        })
    }

    #[inline]
    pub fn make_ordered_list(
        start_number: usize,
        items: Vec<ListItem>,
        tight: bool,
        id: usize,
    ) -> Block {
        Block::List(List::Ordered(OrderedList {
            tight,
            start_number,
            items,
            id,
        }))
    }

    #[inline]
    pub fn make_bullet_list(items: Vec<ListItem>, tight: bool, id: usize) -> Block {
        Block::List(List::Bullet(BulletList { tight, items, id }))
    }

    #[inline]
    pub fn make_list() -> Block {
        todo!()
    }

    #[inline]
    pub fn make_code<T: Into<Option<StrRange>>, A: Into<Option<String>>>(
        range: T,
        range_meta: A,
        lang: Lang,
        id: usize,
    ) -> Block {
        let meta = CodeMeta {
            lang,
            info: range_meta.into(),
        };

        Block::FencedCode(Code {
            meta,
            text: range.into(),
            id,
        })
    }

    #[inline]
    pub fn make_indented_code<T: Into<Box<[StrRange]>>>(indents: T, id: usize) -> Block {
        Block::IndentedCode(IndentCode {
            indents: indents.into(),
            id,
        })
    }

    #[inline]
    pub fn make_heading(range: impl Into<Option<StrRange>>, level: u8, id: usize) -> Block {
        let heading_level = HeadingLevel::new(level);

        Block::Heading(Heading {
            level: heading_level,
            text: range.into(),
            id,
        })
    }

    #[inline]
    pub fn make_style_break(id: usize) -> Block {
        Block::StyleBreak(Break { id })
    }

    fn test(self, data: &[u8]) {
        match self {
            Self::Paragraph(p) => {
                p.text.map(|x| dbg!(x.resolve(data)));
            }

            Self::Heading(p) => {
                p.text.map(|x| dbg!(x.resolve(data)));
            }
            Self::FencedCode(p) => {
                p.text.map(|x| dbg!(x.resolve(data)));
            }
            Self::IndentedCode(p) => {
                p.indents.into_iter().for_each(|x| {
                    dbg!(x.resolve(data));
                });
            }
            Self::Blockquote(p) => p.text.map_or((), |x| dbg!(x.test(data))),

            _ => {}
        }
    }
}

pub(crate) struct BlockParser {
    col: Vec<Block>,
    id: usize,
}

impl BlockParser {
    pub fn new() -> Self {
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

    pub fn block(&mut self, walker: &mut Walker<'_>) -> Block {
        let Some(char) = walker.next() else {
            return Block::Eof;
        };

        let pred = |x: u8| (x == ASTERISK) | (x == LINE) | (x == UNDERSCORE);

        if pred(char) {
            match self.style_break(walker) {
                None => {}
                Some(sb) => return sb,
            }
        }

        match char {
            // Heading
            HASH => match self.heading(walker) {
                None => self.paragraph(walker),
                Some(val) => val,
            },

            // Blockquote
            GREATER_THAN => self.blockquote(walker),
            // Fenced code
            BACKTICK => self.fenced_code::<BACKTICK>(walker),

            // Fenced code again!
            TILDE => self.fenced_code::<TILDE>(walker),

            // Indented code
            SPACE => match self.indented_code(walker) {
                None => {
                    walker.retreat(1);
                    self.paragraph(walker)
                }

                Some(block) => block,
            },

            char if char.is_ascii_digit() => {
                let start = str::from_utf8(&[char])
                    .expect("should always be correct utf-8")
                    .parse::<usize>()
                    .expect("should be a correct number in string form");

                self.ordered_list(start, walker)
            }

            _ => self.paragraph(walker),
        }
    }

    pub fn paragraph(&mut self, walker: &mut Walker<'_>) -> Block {
        let initial = walker.position();

        while let Some(char) = walker.next() {
            match char {
                ch if (ch == NEWLINE) && walker.is_next_char(NEWLINE) => break,

                NEWLINE if walker.is_next_char(EQUALS) => {
                    if let Some(block) = self.handle_special_heading(walker, initial) {
                        return block;
                    }
                }

                BACKTICK => {
                    // let pos = walker.position();
                    let amnt_of_backticks = walker.till_not(BACKTICK);

                    if amnt_of_backticks >= 2 {
                        walker.retreat(amnt_of_backticks + 1);
                        break;
                    }
                }

                _ => {}
            }
        }

        Block::make_paragraph(StrRange::new(initial, walker.position()), self.get_new_id())
    }

    pub fn blockquote(&mut self, walker: &mut Walker<'_>) -> Block {
        let id = self.get_new_id();
        let level = walker.till_not(GREATER_THAN);
        let initial = walker.position();

        let space = usize::from(walker.is_next_char(SPACE));

        while let Some(char) = walker.next() {
            match char {
                NEWLINE => {
                    if check_for_possible_new_block(walker) {
                        println!("Woahl");
                        // walker.advance(1);
                        break;
                    }
                }

                GREATER_THAN => {
                    let amnt_of = walker.till_not(GREATER_THAN);

                    if amnt_of > level {
                        walker.retreat(amnt_of + 1);
                        break;
                    }

                    let advance = usize::from(walker.is_next_char(SPACE));
                    walker.advance(advance);
                }

                _ => {}
            }
        }

        let piece = walker
            .data()
            .get(initial..walker.position())
            .expect("this access should be in bounds");

        let mut new_walker = Walker::new(piece);
        let inner = match self.block(&mut new_walker) {
            Block::Eof => None,

            mut val => {
                val.adjust_range(initial + space, initial);

                Some(val)
            }
        };

        Block::make_blockquote(inner, id, level)
    }

    pub fn fenced_code<const CHAR: u8>(&mut self, walker: &mut Walker<'_>) -> Block {
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
            }

            if char == NEWLINE {
                let range = StrRange::new(pos, walker.position() - 1);

                let mut split = range.resolve(walker.data()).split(',');

                lang = Lang::recognize(
                    split
                        .next()
                        .expect("the first part of a `Split` iterator should be here"),
                );
                info = split.next().map(ToOwned::to_owned);

                break;
            }
        }

        let code_start = walker.position();
        let mut code_end = walker.position();

        while let Some(_char) = walker.next() {
            if walker.is_next_char(CHAR) {
                let amnt_of = walker.till_not(CHAR);

                if amnt_of >= amnt_of_backticks {
                    code_end = walker.position() - amnt_of;
                    walker.advance(1);
                    break;
                }
            }
        }

        Block::make_code(
            StrRange::new(code_start, code_end),
            info,
            lang,
            self.get_new_id(),
        )
    }

    pub fn indented_code(&mut self, walker: &mut Walker<'_>) -> Option<Block> {
        let amnt_of_spaces = walker.till_not(SPACE);

        if amnt_of_spaces < 3 {
            walker.retreat(amnt_of_spaces);
            return None;
        }

        let mut ranges = Vec::with_capacity(BLOCK_VEC_PREALLOCATION);

        let range = walker.till_inclusive(NEWLINE);
        ranges.push(range);

        walker.advance(1);
        Self::indented_code_inner(walker, &mut ranges);

        Block::make_indented_code(ranges, self.get_new_id()).into()
    }

    fn indented_code_inner(walker: &mut Walker<'_>, accum: &mut Vec<StrRange>) {
        let amnt_of_spaces = walker.till_not(SPACE);

        if amnt_of_spaces < 4 {
            walker.retreat(amnt_of_spaces);
            return;
        }

        let range = walker.till_inclusive(NEWLINE);

        walker.advance(1);
        accum.push(range);

        Self::indented_code_inner(walker, accum);
    }

    pub fn heading(&mut self, walker: &mut Walker<'_>) -> Option<Block> {
        let level = if walker.is_next_char(HASH) {
            let temp = walker.till_not(HASH);

            if temp > 5 {
                walker.retreat(temp + 1);
                return None;
            }

            u8::try_from(temp).unwrap_or_else(|_| unreachable!()) + 1
        } else {
            1
        };

        if walker.is_next_char(SPACE) {
            walker.advance(1);
        } else {
            walker.retreat(level as usize);
            let para = self.paragraph(walker);
            return para.into();
        }

        let range = walker.till_inclusive(NEWLINE);
        walker.advance(1);

        Block::make_heading(range, level, self.get_new_id()).into()
    }

    #[inline]
    pub fn special_heading(start: usize, end: usize, id: usize) -> Block {
        Block::make_heading(StrRange::new(start, end), 1, id)
    }

    fn handle_special_heading(&mut self, walker: &mut Walker<'_>, initial: usize) -> Option<Block> {
        let mut heading = true;
        let pos = walker.position();
        while let Some(char) = walker.next() {
            match char {
                NEWLINE => break,

                EQUALS => {}

                _ => {
                    heading = false;
                    break;
                }
            }
        }

        if heading {
            Self::special_heading(initial - 1, pos - 1, self.get_new_id()).into()
        } else {
            None
        }
    }

    pub fn style_break(&mut self, walker: &mut Walker<'_>) -> Option<Block> {
        let initial = walker.position();

        let pred = |x| (x == ASTERISK) | (x == LINE) | (x == UNDERSCORE);

        if walker.is_next_pred(pred) {
            if walker.peek(2).is_some_and(pred) {
                return None;
            } else {
                walker.advance(1);
            }
        } else {
            return None;
        }

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

        Block::make_style_break(self.get_new_id()).into()
    }

    pub fn ordered_list(&mut self, start: usize, walker: &mut Walker<'_>) -> Block {
        if is_ordered_list_indicator(walker) {
            walker.advance(1);
        } else {
            walker.retreat(1);

            return self.paragraph(walker);
        }

        let initial = walker.position();
        while let Some(char) = walker.next() {
            if char == NEWLINE && check_for_possible_new_block(walker) {
                break;
            }

            if char == NEWLINE && walker.is_next_pred(|x| x.is_ascii_digit()) {
                walker.advance(1);
                if is_ordered_list_indicator(walker) {
                    break;
                }
            }
        }

        let mut new_walker = Walker::new(
            walker
                .data()
                .get(initial..walker.position() - 1)
                .expect("always present"),
        );

        let mut block = self.block(&mut new_walker);
        block.adjust_range(initial + 1, initial);

        let mut construct = OListConstructor::new(start - 1);
        let mut tight = true;
        construct.push_item(block);

        self.ordered_list_inner(walker, &mut construct, &mut tight);

        construct.finish(self.get_new_id(), tight)
    }

    fn ordered_list_inner(
        &mut self,
        walker: &mut Walker<'_>,
        accum: &mut OListConstructor,
        tightness: &mut bool,
    ) {
        if is_ordered_list_indicator(walker) {
            walker.advance(1);
        } else {
            walker.retreat(1);
            return;
        }

        let initial = walker.position();
        while let Some(char) = walker.next() {
            if char == NEWLINE {
                if check_for_possible_new_block(walker) {
                    break;
                }

                if walker.is_next_char(NEWLINE) {
                    *tightness = false;
                    walker.advance(1);
                }

                walker.advance(1);
                if is_ordered_list_indicator(walker) {
                    break;
                } else {
                    walker.retreat(1);
                }
            }
        }

        let mut new_walker = Walker::new(
            walker
                .data()
                .get(initial + 1..walker.position())
                .expect("always present"),
        );

        let mut block = self.block(&mut new_walker);
        block.adjust_range(initial, initial);
        accum.push_item(block);

        self.ordered_list_inner(walker, accum, tightness);
    }
}

fn check_for_possible_new_block(walker: &mut Walker<'_>) -> bool {
    let next = match walker.peek(0) {
        None => return false,
        Some(val) => val,
    };

    match next {
        NEWLINE => {
            walker.advance(1);
            true
        }

        BACKTICK => {
            // let pos = walker.position();
            let amnt_of_backticks = walker.till_not(BACKTICK);

            if amnt_of_backticks < 3 {
                walker.retreat(amnt_of_backticks);
                true
            } else {
                false
            }
        }

        HASH => {
            // let pos = walker.position();
            let amnt_of_hashes = walker.till_not(HASH);
            let is_after_space = walker.is_next_char(SPACE);

            if 6 > amnt_of_hashes || is_after_space {
                walker.retreat(amnt_of_hashes);
                true
            } else {
                false
            }
        }

        _ => false,
    }
}

// god remake this
fn is_ordered_list_indicator(walker: &mut Walker<'_>) -> bool {
    if !walker.is_next_pred(|x: u8| (x == DOT) || (x == RIGHT_PAREN))
        || walker.peek(1) != Some(SPACE)
    {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {

    use super::BlockParser;
    use crate::{block_parser::Block, walker::Walker};

    #[test]
    fn complete() {
        let data = concat!(
            "> Blockquote\n",
            ">BlockquoteNoSpace\n",
            "# Heading\n",
            "#BrokenHeading\n",
            "```rust,some_meta_data=noumea :3\n",
            "panic!()\n",
            "```\n",
            "    Indented code!\n",
            "--*\n",
            "Heading with equals\n",
            "======\n",
            "and let's have a nice paragraph\n",
        )
        .as_bytes();

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        match parser.block(&mut walker) {
            Block::Blockquote(bq) => {
                match *bq.text.expect("no inner element") {
                    Block::Paragraph(para) => {
                        let text = para.text.expect("text was not present");
                        assert!("Blockquote\n>BlockquoteNoSpace\n" == text.resolve(data));
                    }

                    _ => panic!("inner block was not a paragraph"),
                };
            }

            _ => panic!("block was not a blockquote"),
        };

        match parser.block(&mut walker) {
            Block::Heading(h) => {
                let text = h.text.expect("no text present in heading");

                assert!(text.resolve(data) == "Heading");
                assert!(u8::from(h.level.0) == 1);
            }

            _ => panic!("block was not a blockquote"),
        };

        let block = dbg!(parser.block(&mut walker));
        let block = dbg!(parser.block(&mut walker));
        let block = dbg!(parser.block(&mut walker));
        let block = dbg!(parser.block(&mut walker));
        let block = dbg!(parser.block(&mut walker));
        let block = dbg!(parser.block(&mut walker));
    }

    #[test]
    fn blockquote() {
        let md = concat!(
            ">>> This is a blockquote\n",
            ">>>> This is an another blockquote\nbut a longer one!",
        )
        .as_bytes();

        let mut parser = BlockParser::new();
        let mut walker = Walker::new(md);

        let val = parser.block(&mut walker);

        let inner = match val {
            Block::Blockquote(q) => *q.text.expect("field not present"),
            _ => panic!("block was not blockquote"),
        };

        match inner {
            Block::Paragraph(para) => {
                let text = para.text.unwrap();

                let resolved = text.resolve(md);

                assert!(dbg!(resolved) == "This is a blockquote\n");
            }

            _ => assert!(false, "block was not paragraph"),
        }

        let val = parser.block(&mut walker);

        let inner = match val {
            Block::Blockquote(q) => *q.text.expect("field not present"),
            _ => panic!("block was not blockquote"),
        };

        match inner {
            Block::Paragraph(para) => {
                let text = para.text.unwrap();

                let resolved = text.resolve(md);
                dbg!(resolved);
                assert!(resolved == "This is an another blockquote\nbut a longer one!");
            }

            _ => assert!(false, "block was not paragraph"),
        }
    }

    #[test]
    fn ordered_list() {
        let data = concat!(
            "1) Niente dei, niente padroni\n",
            "2) No gods, no masters\n",
            "3) Ni dieu, ni maitre\n",
            "4) Ani boga, ani pana\n",
        )
        .as_bytes();

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        match dbg!(parser.block(&mut walker)) {
            Block::List(ord) => match ord {
                super::List::Ordered(order) => {
                    let items = order.items.into_iter();

                    items.for_each(|item| {
                        match *item.item {
                            Block::Paragraph(parap) => {
                                println!("StrRange: {:#?}", parap.text.as_ref().unwrap().get());
                                println!("Resolved text:\n{:#?}", parap.text.unwrap().resolve(data))
                            }

                            _ => panic!("was not paragraph"),
                        };
                    });
                }

                _ => panic!("list was not ordered"),
            },

            _ => panic!("block was not an ordered list"),
        };
    }

    #[test]
    fn code() {
        let data = concat!("```rust\n", "#[no_std]\n", "```").as_bytes();

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        let block = match parser.block(&mut walker) {
            Block::FencedCode(fc) => fc,

            _ => panic!("block was not fenced code"),
        };

        assert!(block.text.expect("text should be here").resolve(data) == "#[no_std]\n");
    }

    #[test]
    fn code_tilde() {
        let data = concat!("~~~rust\n", "#[no_std]\n", "~~~").as_bytes();

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        let block = match parser.block(&mut walker) {
            Block::FencedCode(fc) => fc,

            _ => panic!("block was not fenced code"),
        };

        assert!(block.text.expect("text should be here").resolve(data) == "#[no_std]\n");
    }

    #[test]
    fn code_indented() {
        let data = concat!(
            "       code line 1\n",
            "       code line 2\n",
            "       code line 3\n",
            "       code line 4\n",
            "       code line 5\n",
        );

        let mut walker = Walker::new(data.as_bytes());
        let mut parser = BlockParser::new();

        let block = parser.block(&mut walker);

        let inner = match block {
            Block::IndentedCode(ic) => ic,
            _ => panic!("block was not indented code"),
        };

        inner
            .indents
            .into_iter()
            .enumerate()
            .map(|(index, val)| (index + 1, val.resolve(data.as_bytes())))
            .for_each(|(index, value)| {
                let test = format!("code line {}", index);

                assert!(test == value, "wrong value at line: {}", index)
            });
    }

    #[test]
    fn heading_simple() {
        let data = "###### une, grande, et indivisible".as_bytes();

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        let block = match parser.block(&mut walker) {
            Block::Heading(h) => h,

            _ => panic!("block was not a heading"),
        };

        assert!(
            u8::from(block.level.0) == 6,
            "invalid level found, was supposed to be 6, is {}",
            block.level.0
        );

        assert!(
            block.text.expect("should be here").resolve(data) == "une, grande, et indivisible",
            "invalid text in heading"
        );
    }

    #[test]
    fn heading_under() {
        let data = concat!("Heading text\n", "======",).as_bytes();

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        let block = match parser.block(&mut walker) {
            Block::Heading(h) => h,

            _ => panic!("block was not a heading"),
        };

        assert!(
            u8::from(block.level.0) == 1,
            "invalid level found, was supposed to be 1, is {}",
            block.level.0
        );

        assert!(
            block.text.expect("should be here").resolve(data) == "Heading text",
            "invalid text in heading"
        );
    }

    #[test]
    fn style_break_simple() {
        let data = concat!("___\n", "---\n", "***\n").as_bytes();

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        match parser.block(&mut walker) {
            Block::StyleBreak(_) => {}

            _ => panic!("block was not style break"),
        };

        match parser.block(&mut walker) {
            Block::StyleBreak(_) => {}

            _ => panic!("block was not style break"),
        };

        match parser.block(&mut walker) {
            Block::StyleBreak(_) => {}

            _ => panic!("block was not style break"),
        };
    }
}
