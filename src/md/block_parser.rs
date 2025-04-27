#![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]

use crate::md::chars::{
    ASTERISK, BACKTICK, DOT, EQUALS, GREATER_THAN, HASH, LINE, NEWLINE, PLUS, RIGHT_PAREN, SPACE,
    TILDE, UNDERSCORE,
};

use crate::md::html_constants::SIMPLE_CONDITIONS;
use crate::walker::{StrRange, Walker};
use core::marker::PhantomData;
use core::num::NonZero;
use core::str;

use super::chars::LESSER_THAN;
use super::html_constants::HTML_ALLOWED_TAGS;

static BLOCK_VEC_PREALLOCATION: usize = 64;

#[derive(Debug)]
pub struct Paragraph {
    text: String,
    id: usize,
}

#[derive(Debug)]
pub struct BlkQt {
    level: BlkQtLevel,
    text: Option<Box<Block<Unparsed>>>,
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

    pub fn push_item(&mut self, item: Block<Unparsed>) {
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

    pub fn finish(self, id: usize, tight: bool) -> Block<Unparsed> {
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
    item: Box<Block<Unparsed>>,
}

#[derive(Debug)]
pub struct Code {
    meta: CodeMeta,
    text: Option<String>,
    id: usize,
}

#[derive(Debug)]
pub struct IndentCode {
    indents: Box<[String]>,
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
    level: Option<HeadingLevel>,
    text: Option<String>,
    id: usize,
}

impl Heading {
    pub fn is_level(&self, cmp: u8) -> bool {
        self.level.is_some_and(|lvl| u8::from(lvl.0) == cmp)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct HeadingLevel(NonZero<u8>);

#[derive(Debug)]
pub struct Break {
    id: usize,
}

impl HeadingLevel {
    pub fn new(level: u8) -> Self {
        debug_assert!(level > 0, "level was lower than 1");

        let nonzero = unsafe { NonZero::new_unchecked(level) };

        Self(nonzero)
    }
}

#[derive(Debug)]
pub struct Parsed;
#[derive(Debug)]
pub struct Unparsed;

#[derive(Debug)]
pub struct HtmlBlock {
    inner: String,
    id: usize,
}

#[derive(Debug)]
pub enum Block<State> {
    Paragraph(Paragraph),
    Blockquote(BlkQt),
    List(List),
    FencedCode(Code),
    IndentedCode(IndentCode),
    Heading(Heading),
    StyleBreak(Break),
    HtmlBlock(HtmlBlock),
    Eof,

    _State(PhantomData<State>),
}

impl Block<Unparsed> {
    #[inline]
    pub fn make_paragraph(text: String, id: usize) -> Block<Unparsed> {
        Block::Paragraph(Paragraph { text, id })
    }

    #[inline]
    pub fn make_blockquote(
        range: impl Into<Option<Block<Unparsed>>>,
        id: usize,
        level: usize,
    ) -> Block<Unparsed> {
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
    ) -> Block<Unparsed> {
        Block::List(List::Ordered(OrderedList {
            tight,
            start_number,
            items,
            id,
        }))
    }

    #[inline]
    pub fn make_bullet_list(items: Vec<ListItem>, tight: bool, id: usize) -> Block<Unparsed> {
        Block::List(List::Bullet(BulletList { tight, items, id }))
    }

    #[inline]
    pub fn make_list() -> Block<Unparsed> {
        todo!()
    }

    #[inline]
    pub fn make_code(
        code: impl Into<Option<String>>,
        meta: impl Into<Option<String>>,
        lang: Lang,
        id: usize,
    ) -> Block<Unparsed> {
        let meta = CodeMeta {
            lang,
            info: meta.into(),
        };

        Block::FencedCode(Code {
            meta,
            text: code.into(),
            id,
        })
    }

    #[inline]
    pub fn make_indented_code<T: Into<Box<[String]>>>(indents: T, id: usize) -> Block<Unparsed> {
        Block::IndentedCode(IndentCode {
            indents: indents.into(),
            id,
        })
    }

    #[inline]
    pub fn make_heading(
        range: impl Into<Option<String>>,
        heading_level: impl Into<Option<u8>>,
        id: usize,
    ) -> Block<Unparsed> {
        let level = match heading_level.into() {
            Some(val) => Some(HeadingLevel::new(val)),

            None => None,
        };

        Block::Heading(Heading {
            level,
            text: range.into(),
            id,
        })
    }

    #[inline]
    pub fn make_style_break(id: usize) -> Block<Unparsed> {
        Block::StyleBreak(Break { id })
    }

    #[inline]
    pub fn make_html_block(inner: String, id: usize) -> Block<Unparsed> {
        Block::HtmlBlock(HtmlBlock { inner, id })
    }
}

#[derive(Debug)]
pub struct Document<State> {
    blocks: Option<Vec<Block<State>>>,
    _phantom: PhantomData<State>,
}

impl Document<Unparsed> {
    pub fn new() -> Self {
        Self {
            blocks: None,
            _phantom: PhantomData::<Unparsed>,
        }
    }

    pub fn add(&mut self, val: Block<Unparsed>) {
        match self.blocks {
            None => {
                let mut vec = Vec::with_capacity(128);

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

pub(crate) struct BlockParser {
    col: Document<Unparsed>,
    id: usize,
}

impl BlockParser {
    /// Creates a new `BlockParser`
    pub fn new() -> Self {
        Self {
            col: Document::default(),
            id: 0,
        }
    }

    /// Populates an document with `Block`s
    /// without parsed inline contents
    pub fn document(mut self, walker: &mut Walker<'_>) -> Document<Unparsed> {
        loop {
            let val = match self.block(walker) {
                Block::Eof => break,
                val => val,
            };

            self.col.add(val);
        }

        self.col
    }

    fn get_new_id(&mut self) -> usize {
        let id = self.id;

        self.id += 1;

        id
    }

    /// Parses some block into a "unparsed state"
    /// meaning the block has text contents that are yet to be parsed
    /// into inlines via a `InlineParser`.
    pub fn block(&mut self, walker: &mut Walker<'_>) -> Block<Unparsed> {
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

            char if is_bullet_list_marker(char) => self.bullet_list(char, walker),

            LESSER_THAN => self.html_block(walker),

            _ => self.paragraph(walker),
        }
    }

    pub fn paragraph(&mut self, walker: &mut Walker<'_>) -> Block<Unparsed> {
        let initial = walker.position();

        while let Some(char) = walker.next() {
            match char {
                ch if (ch == NEWLINE) && walker.is_next_char(NEWLINE) => break,

                NEWLINE => {
                    if walker.is_next_char(EQUALS) {
                        if let Some(block) = self.handle_special_heading(walker, initial) {
                            return block;
                        }
                    }

                    if check_for_possible_new_block(walker) {
                        break;
                    }
                }

                _ => {}
            }
        }

        let mut string = String::with_capacity(walker.position() - initial);
        let mut iter = walker.get(initial, walker.position()).chars();

        if let Some(char) = iter.next() {
            if char != ' ' {
                string.push(char);
            }
        }

        iter.filter(|char| char != &'>')
            .map(|char| if char == '\n' { ' ' } else { char })
            .for_each(|char| String::push(&mut string, char));

        if let Some(" ") = string.get(string.len() - 1..) {
            let _space = string.pop();
        }

        Block::make_paragraph(string, self.get_new_id())
    }

    pub fn blockquote(&mut self, walker: &mut Walker<'_>) -> Block<Unparsed> {
        let id = self.get_new_id();
        let level = walker.till_not(GREATER_THAN);
        let initial = walker.position();
        let space: usize = walker.is_next_char(SPACE).into();

        while let Some(char) = walker.next() {
            match char {
                NEWLINE => {
                    if check_for_possible_new_block(walker) {
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

        let mut new_walker = walker.walker_from_initial(initial);

        let inner = match self.block(&mut new_walker) {
            Block::Eof => None,

            val => Some(val),
        };

        Block::make_blockquote(inner, id, level)
    }

    pub fn fenced_code<const CHAR: u8>(&mut self, walker: &mut Walker<'_>) -> Block<Unparsed> {
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

        let string = String::from(walker.get(code_start, code_end));

        Block::make_code(string, info, lang, self.get_new_id())
    }

    pub fn indented_code(&mut self, walker: &mut Walker<'_>) -> Option<Block<Unparsed>> {
        let amnt_of_spaces = walker.till_not(SPACE);

        if amnt_of_spaces < 3 {
            walker.retreat(amnt_of_spaces);
            return None;
        }

        let mut lines = Vec::with_capacity(BLOCK_VEC_PREALLOCATION);

        let entry = String::from(walker.till_inclusive(NEWLINE));
        lines.push(entry);

        walker.advance(1);

        loop {
            let amnt_of_spaces = walker.till_not(SPACE);

            if amnt_of_spaces < 4 {
                walker.retreat(amnt_of_spaces);
                break;
            }

            let entry = walker.till_inclusive(NEWLINE);
            lines.push(String::from(entry));
            walker.advance(1);
        }

        Block::make_indented_code(lines.into_boxed_slice(), self.get_new_id()).into()
    }

    pub fn heading(&mut self, walker: &mut Walker<'_>) -> Option<Block<Unparsed>> {
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
            return self.paragraph(walker).into();
        }

        let range = String::from(walker.till_inclusive(NEWLINE));
        walker.advance(1);

        Block::make_heading(range, level, self.get_new_id()).into()
    }

    #[inline]
    /// helper function for creating a heading
    fn special_heading(
        start: usize,
        end: usize,
        walker: &mut Walker<'_>,
        id: usize,
    ) -> Block<Unparsed> {
        let string = String::from(walker.get(start, end));
        Block::make_heading(string, 1, id)
    }

    // helper function for a heading
    // made by using equals under some text
    // like:
    // ```
    // My heading
    // ==========
    // ```
    fn handle_special_heading(
        &mut self,
        walker: &mut Walker<'_>,
        initial: usize,
    ) -> Option<Block<Unparsed>> {
        let mut heading = true;
        let pos = walker.position();
        while let Some(char) = walker.next() {
            match char {
                NEWLINE => {
                    walker.retreat(1);
                    break;
                }

                EQUALS => {}

                _ => {
                    heading = false;
                    break;
                }
            }
        }

        if heading {
            Self::special_heading(initial - 1, pos - 1, walker, self.get_new_id()).into()
        } else {
            None
        }
    }

    pub fn style_break(&mut self, walker: &mut Walker<'_>) -> Option<Block<Unparsed>> {
        let initial = walker.position();

        let pred = |x| (x == ASTERISK) | (x == LINE) | (x == UNDERSCORE);

        if walker.is_next_pred(pred) {
            if walker.peek(2).is_some_and(pred) {
                return None;
            }

            walker.advance(1);
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

    pub fn ordered_list(&mut self, start: usize, walker: &mut Walker<'_>) -> Block<Unparsed> {
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

        let mut new_walker = walker.walker_from_initial(initial);
        let block = self.block(&mut new_walker);

        let mut construct = OListConstructor::new(start - 1);
        let mut tight = true;
        construct.push_item(block);

        walker.advance(1);

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
                }

                walker.retreat(1);
            }
        }

        let mut new_walker = walker.walker_from_initial(initial + 1);
        let block = self.block(&mut new_walker);
        accum.push_item(block);

        walker.advance(1);

        self.ordered_list_inner(walker, accum, tightness);
    }

    fn bullet_list(&mut self, delim: u8, walker: &mut Walker<'_>) -> Block<Unparsed> {
        debug_assert!(
            matches!(delim, PLUS | ASTERISK | LINE),
            "char given to `bullet_list` was not a `+`, a `*` nor a `-`"
        );

        let initial = walker.position();
        while let Some(char) = walker.next() {
            if char == NEWLINE && check_for_possible_new_block(walker) {
                break;
            }

            if char == NEWLINE
                && walker.is_next_pred(is_bullet_list_marker)
                && walker.peek(1) == Some(SPACE)
            {
                break;
            }
        }

        let mut list_items = Vec::new();

        let mut new_walker = walker.walker_from_initial(initial);
        let block = self.block(&mut new_walker);

        let mut tight = true;

        list_items.push(ListItem {
            number: None,
            item: Box::new(block),
        });

        self.bullet_list_inner(walker, &mut list_items, delim, &mut tight);

        Block::make_bullet_list(list_items, tight, self.get_new_id())
    }

    fn bullet_list_inner(
        &mut self,
        walker: &mut Walker<'_>,
        accum: &mut Vec<ListItem>,
        delim: u8,
        tight: &mut bool,
    ) {
        debug_assert!(
            matches!(delim, PLUS | ASTERISK | LINE),
            "char given to `bullet_list_inner` was not a `+`, a `*` nor a `-`"
        );

        if !walker.is_next_pred(is_bullet_list_marker) && walker.peek(0) != Some(delim) {
            return;
        }

        let initial = walker.position();
        while let Some(char) = walker.next() {
            if char == NEWLINE {
                if check_for_possible_new_block(walker) {
                    break;
                }

                if walker.is_next_char(NEWLINE) {
                    *tight = false;
                    walker.advance(1);
                }

                if walker.peek(0) != Some(delim)
                    && walker.peek(0).is_some_and(is_bullet_list_marker)
                {
                    walker.retreat(1);
                    return;
                }

                if walker.is_next_pred(|x| x == delim) && walker.peek(1) == Some(SPACE) {
                    break;
                }
            }
        }

        let mut new_walker = walker.walker_from_initial(initial + 1);
        let block = self.block(&mut new_walker);

        accum.push(ListItem {
            number: None,
            item: Box::new(block),
        });

        self.bullet_list_inner(walker, accum, delim, tight);
    }

    pub fn html_block(&mut self, walker: &mut Walker<'_>) -> Block<Unparsed> {
        let initial = walker.position();

        for (index, cond) in SIMPLE_CONDITIONS.into_iter().enumerate() {
            if walker.find_string(cond[0]) {
                // the `!` must be followed by an ascii character
                if index == 7 {
                    if !walker.is_next_pred(|x| x.is_ascii_alphabetic()) {
                        walker.retreat(1);
                        return self.paragraph(walker);
                    }
                }

                let first_char_of_end = cond[1].as_bytes().get(0).expect("must be here");

                'inner: while let Some(char) = walker.next() {
                    if char == *first_char_of_end {
                        let result = &cond[1][1..];

                        if walker.find_string(result) {
                            break 'inner;
                        }
                    };
                }

                let _ = walker.till(NEWLINE);
                let string = String::from(walker.get(initial - 1, walker.position() - 1));
                return Block::make_html_block(string, self.get_new_id());
            }
        }

        if walker.is_next_char(b'/') {
            walker.advance(1);
            for tag in HTML_ALLOWED_TAGS.into_iter() {
                if walker.find_string(tag) {
                    break;
                }
            }

            while let Some(char) = walker.next() {
                if (char == b'/' && walker.is_next_char(GREATER_THAN))
                    || walker.is_next_char(GREATER_THAN)
                {
                    break;
                }
            }
        }

        let _ = walker.till(NEWLINE);

        let string = String::from(walker.string_from_offset(initial - 1));

        Block::make_html_block(string, self.get_new_id())
    }
}

fn check_for_possible_new_block(walker: &mut Walker<'_>) -> bool {
    let next = match walker.peek(0) {
        None => return false,
        Some(val) => val,
    };

    // let cur_part = walker.get(walker.position(), walker.data().len() - 1);
    // dbg!(cur_part);
    // let n = &[next];
    // let cur_char = str::from_utf8(n).unwrap();
    // dbg!(cur_char);

    match next {
        NEWLINE => {
            walker.advance(1);
            true
        }

        BACKTICK => {
            // let pos = walker.position();
            let amnt_of_backticks = walker.till_not(BACKTICK);

            if amnt_of_backticks <= 3 {
                walker.retreat(amnt_of_backticks);

                true
            } else {
                false
            }
        }

        HASH => {
            let amnt_of_hashes = walker.till_not(HASH);
            let is_after_space = walker.is_next_char(SPACE);

            if 6 > amnt_of_hashes && is_after_space {
                walker.retreat(amnt_of_hashes);
                true
            } else {
                false
            }
        }

        char if char.is_ascii_digit() => {
            walker.advance(1);
            if is_ordered_list_indicator(walker) {
                walker.retreat(1);
                true
            } else {
                walker.retreat(1);
                false
            }
        }

        char if is_bullet_list_marker(char) => {
            walker.advance(1);
            let bool = walker.is_next_char(SPACE);

            walker.retreat(1);

            bool
        }
        _ => false,
    }
}

/// Used after a numeric character
/// returns true if the 2 next characters are either `. ` or `) `.
/// does not advance the position of the walker.
fn is_ordered_list_indicator(walker: &mut Walker<'_>) -> bool {
    if !walker.is_next_pred(|x: u8| (x == DOT) || (x == RIGHT_PAREN))
        || walker.peek(1) != Some(SPACE)
    {
        return false;
    }

    true
}

fn is_bullet_list_marker(victim: u8) -> bool {
    matches!(victim, ASTERISK | LINE | PLUS)
}

fn is_useless_char(char: u8) -> bool {
    match char {
        GREATER_THAN => true,

        _ => false,
    }
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
            "1) Order 1\n",
            "2) Order 2\n",
            "3) Order 3\n",
            "4) Order 4\n",
            "+ Meow\n",
            "+ Awrff\n",
            "+ Bark\n"
        );

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        match parser.block(&mut walker) {
            Block::Blockquote(bq) => {
                match *bq.text.expect("no inner element") {
                    Block::Paragraph(para) => {
                        let text = para.text;
                        assert!(
                            "Blockquote BlockquoteNoSpace" == text,
                            "invalid text, was: {text}"
                        );
                    }

                    _ => panic!("inner block was not a paragraph"),
                };
            }

            any => panic!("block was not a blockquote, was: {:#?}", any),
        };

        match parser.block(&mut walker) {
            Block::Heading(h) => {
                let text = h.text.expect("no text present in heading");

                assert!(text == "Heading");
                assert!(h.level.map_or(false, |x| u8::from(x.0) == 1));
            }

            any => panic!("block was not a blockquote, was: {:#?}", any),
        };

        match parser.block(&mut walker) {
            Block::Paragraph(para) => {
                assert!(para.text == "#BrokenHeading")
            }

            any => panic!("block was not a paragraph, was: {:#?}", any),
        }

        match parser.block(&mut walker) {
            Block::FencedCode(code) => {
                match code.meta.info {
                    Some(info) => assert!("some_meta_data=noumea :3" == info, "invalid meta data"),
                    _ => panic!("no metadata was found"),
                }

                match code.meta.lang {
                    super::Lang::Rust => {}

                    lang => panic!("invalid language recognised: {lang:#?}"),
                }

                assert!(
                    code.text.is_some_and(|str| str == "panic!()\n"),
                    "wrongly read code block"
                )
            }

            any => panic!("block was not fenced code, was: {:#?}", any),
        };

        match parser.block(&mut walker) {
            Block::IndentedCode(icode) => {
                let text = icode.indents.get(0).expect("only indent was not present");

                assert!(text == "Indented code!");
            }

            any => panic!("block was not `IndentedCode`, was: {:#?}", any),
        }

        match parser.block(&mut walker) {
            Block::StyleBreak(_) => {}

            any => panic!("block was not `StyleBreak`, was: {:#?}", any),
        };

        match parser.block(&mut walker) {
            Block::Heading(hd) => {
                assert!(hd.is_level(1), "wrong heading level");
                assert!(
                    hd.text.is_some_and(|x| x == "Heading with equals"),
                    "invalid heading text"
                );
            }

            any => panic!("block was not `Heading`, was: {:#?}", any),
        }

        match parser.block(&mut walker) {
            Block::Paragraph(para) => assert!(
                para.text == "and let's have a nice paragraph",
                "invalid paragraph text: {0}",
                para.text
            ),

            any => panic!("block was not `Paragraph`, was: {:#?}", any),
        };

        match dbg!(parser.block(&mut walker)) {
            Block::List(ord) => match ord {
                super::List::Ordered(order) => {
                    let items = order.items.into_iter();

                    items.for_each(|item| {
                        match *item.item {
                            Block::Paragraph(parap) => {
                                println!("text:\n{:#?}", parap.text)
                            }

                            _ => panic!("was not paragraph"),
                        };
                    });
                }

                _ => panic!("list was not ordered"),
            },

            _ => panic!("block was not an ordered list"),
        };

        match parser.block(&mut walker) {
            Block::List(ls) => match ls {
                super::List::Bullet(b) => {
                    b.items.into_iter().for_each(|x| {
                        let string = match *x.item {
                            Block::Paragraph(p) => p.text,

                            any => panic!("not paragraph, is: {any:#?}"),
                        };

                        dbg!(string);
                    });
                }

                any => panic!("not bullet list, is: {any:#?} "),
            },

            any => panic!("not list, is: {any:#?}"),
        };

        // match parser.block(&mut walker) {
        //     Block::Eof => {}

        //     any => panic!("not EOF, block returned was: {:#?}", any),
        // };
    }

    #[test]
    fn blockquote() {
        let md = concat!(
            ">>> This is a blockquote\n",
            ">>>> This is an another blockquote\nbut a longer one!",
        );

        let mut parser = BlockParser::new();
        let mut walker = Walker::new(md);

        let val = parser.block(&mut walker);

        let inner = match val {
            Block::Blockquote(q) => *q.text.expect("field not present"),
            _ => panic!("block was not blockquote"),
        };

        match inner {
            Block::Paragraph(para) => {
                let text = para.text;

                assert!(text == "This is a blockquote");
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
                let text = para.text;

                assert!(text == "This is an another blockquote but a longer one!");
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
        );

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        match dbg!(parser.block(&mut walker)) {
            Block::List(ord) => match ord {
                super::List::Ordered(order) => {
                    let items = order.items.into_iter();

                    items.for_each(|item| {
                        match *item.item {
                            Block::Paragraph(parap) => {
                                println!("text:\n{:#?}", parap.text)
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
    fn bullet_list() {
        let data = concat!("+ Meow\n", "+ Awrff\n", "+ Bark\n");

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        match parser.block(&mut walker) {
            Block::List(ls) => match ls {
                super::List::Bullet(b) => {
                    b.items.into_iter().for_each(|x| {
                        let string = match *x.item {
                            Block::Paragraph(p) => p.text,

                            _ => panic!("not paragraph"),
                        };

                        dbg!(string);
                    });
                }

                _ => panic!("not bullet list"),
            },

            _ => panic!("not list"),
        };
    }

    #[test]
    fn code() {
        let data = concat!("```rust\n", "#[no_std]\n", "```");

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        let block = match parser.block(&mut walker) {
            Block::FencedCode(fc) => fc,

            _ => panic!("block was not fenced code"),
        };

        assert!(block.text.expect("text should be here") == "#[no_std]\n");
    }

    #[test]
    fn code_tilde() {
        let data = concat!("~~~rust\n", "#[no_std]\n", "~~~");

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        let block = match parser.block(&mut walker) {
            Block::FencedCode(fc) => fc,

            _ => panic!("block was not fenced code"),
        };

        assert!(block.text.expect("text should be here") == "#[no_std]\n");
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

        let mut walker = Walker::new(data);
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
            .map(|(index, val)| (index + 1, val))
            .for_each(|(index, value)| {
                let test = format!("code line {}", index);

                assert!(test == value, "wrong value at line: {}", index)
            });
    }

    #[test]
    fn heading_simple() {
        let data = "###### une, grande, et indivisible";

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        let block = match parser.block(&mut walker) {
            Block::Heading(h) => h,

            _ => panic!("block was not a heading"),
        };

        assert!(
            block.is_level(6),
            "invalid level found, was supposed to be 6, is {:#?}",
            block.level
        );

        assert!(
            block.text.expect("should be here") == "une, grande, et indivisible",
            "invalid text in heading"
        );
    }

    #[test]
    fn heading_under() {
        let data = concat!("Heading text\n", "======",);

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        let block = match parser.block(&mut walker) {
            Block::Heading(h) => h,

            _ => panic!("block was not a heading"),
        };

        assert!(
            block.is_level(1),
            "invalid level found, was supposed to be 1, is {:#?}",
            block.level
        );

        let text = block.text.expect("should be here");
        assert!(
            text == "Heading text",
            "invalid text in heading, was: {text:?}"
        );
    }

    #[test]
    fn style_break_simple() {
        let data = concat!("___\n", "---\n", "***\n");

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

    #[test]
    fn html_blocks() {
        let data = concat!(
            "<pre va>this is some serious content</pre>\n",
            "<script \t this is some serious content 2</script>\n",
            "<textarea \t this is some serious content</textarea>\n",
            "<style \t this is some serious content</style>\n",
            "<!-- html comment -->\n",
            "<? whatever ?>\n",
            "<!block>\n",
            "<![CDATA[ \"L'Alsace et la Lorraine\" ]]>\n",
            "</address \t>\n",
            "<vabank"
        );

        let mut walker = Walker::new(data);
        let parser = BlockParser::new();

        let document = parser.document(&mut walker);

        println!("{:#?}", document);
    }
}
