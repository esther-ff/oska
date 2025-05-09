use crate::md::{
    Document,
    blocks::{Block, Parsed, Unparsed},
    inlines::{EmphasisChar, Image, Inline, Inlines, Text},
    walker::{StrRange, Walker},
};

use core::cell::{Cell, Ref, RefCell, RefMut};
use core::fmt::Debug;
use unicode_categories::UnicodeCategories;

/// A trait representing a parser for inline elements
/// such as emphases or links.
pub trait InlineParser {
    fn parse(&mut self, item: Block<Unparsed>) -> Block<Parsed>;
    fn parse_doc(&mut self, doc: Document<Unparsed>) -> Document<Parsed>;
    fn parse_inlines(&mut self, src: &str) -> Inlines;
}

/// Default parser for Inlines
pub struct DefInlineParser;

impl InlineParser for DefInlineParser {
    fn parse_inlines(&mut self, src: &str) -> Inlines {
        let mut inl = Inlines::new();
        let mut walker = Walker::new(src);
        let arena = Arena::new();

        tokenize(&mut walker, inl.inner(), &arena);

        let mut last = arena.previous();

        while let Some(inner) = last {
            println!("{:#?}", &*inner.val.borrow());
            last = inner.prev.get();
        }

        dbg!(&inl);
        inl
    }

    fn parse(&mut self, item: Block<Unparsed>) -> Block<Parsed> {
        todo!()
    }

    fn parse_doc(&mut self, doc: Document<Unparsed>) -> Document<Parsed> {
        todo!()
    }
}

// Arena for the delimeters
struct Arena<'a, T> {
    inner: RefCell<Chunks<T>>,
    prev: Cell<Option<&'a T>>,
}

struct Chunks<T> {
    cur: Vec<T>,
    rst: Vec<Vec<T>>,
}

impl<T> Chunks<T> {
    const START_SIZE: usize = 1024 * 4;
    fn new() -> Self {
        assert_ne!(size_of::<T>(), 0);

        Self {
            cur: Vec::with_capacity(Self::START_SIZE / size_of::<T>()),
            rst: Vec::new(),
        }
    }
}

impl<'a, T> Arena<'a, T> {
    fn new() -> Arena<'a, T> {
        Self {
            inner: RefCell::new(Chunks::new()),
            prev: Cell::new(None),
        }
    }

    fn previous(&self) -> Option<&'a T> {
        self.prev.get()
    }

    fn alloc(&self, item: T) -> &T {
        let mut chunk = self.inner.borrow_mut();
        let cur_len = chunk.cur.len();

        let reff = if cur_len < chunk.cur.capacity() {
            chunk.cur.push(item);

            unsafe { &*chunk.cur.as_ptr().add(cur_len) }
        } else {
            let mut new_chunk = Vec::with_capacity(chunk.cur.capacity());
            new_chunk.push(item);
            let old_chunk = core::mem::replace(&mut chunk.cur, new_chunk);
            chunk.rst.push(old_chunk);
            unsafe { &*chunk.cur.as_ptr() }
        };

        self.prev.replace(Some(reff));

        reff
    }
}

impl<'a, T> Arena<'a, Node<'a, T>> {
    fn to_list(&'a self, val: T) {
        let old = self.previous();
        let new = self.alloc(Node::new(val, None, old));

        if let Some(node) = old {
            node.next.set(Some(new));
        }
    }
}

struct Node<'a, T> {
    val: RefCell<T>,
    prev: Cell<Option<&'a Node<'a, T>>>,
    next: Cell<Option<&'a Node<'a, T>>>,
}

impl<'a, T> Node<'a, T> {
    fn new<A, B>(val: T, next: A, prev: B) -> Self
    where
        A: Into<Option<&'a Self>>,
        B: Into<Option<&'a Self>>,
    {
        Node {
            val: RefCell::new(val),
            next: Cell::new(next.into()),
            prev: Cell::new(prev.into()),
        }
    }
}

impl<T: Debug> Debug for Node<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Node")
            .field("val", &self.val)
            .field("prev", &self.prev.get())
            .field("next", &self.prev.get())
            .finish()
    }
}

#[derive(Debug)]
enum Ability {
    Opener,
    Closer,
    Both,
    None,
    NotImportant,
}

#[derive(Debug)]
struct Token {
    // Delimiter of the token
    char: &'static str,

    // Amount of the delimiter
    amount: usize,

    // Position
    pos: TokenPos,

    // Node it is pointing to
    node: Cell<Option<*mut Inline>>,

    // Whether the token is a closer or opener
    ability: Ability,

    // Whether the token is closed
    closed: bool,
}

#[derive(Debug)]
struct TokenPos {
    start: usize,
    end: usize,
}

impl Token {
    fn new(
        char: &'static str,
        amount: usize,
        pos: TokenPos,
        node: Option<*mut Inline>,
        ability: Ability,
    ) -> Self {
        Self {
            char,
            amount,
            pos,
            node: Cell::new(node),
            ability,
            closed: false,
        }
    }
}

impl TokenPos {
    fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    fn tuple(&self) -> (usize, usize) {
        (self.start, self.end)
    }
}

struct Cursor<'a> {
    cur: Option<&'a Node<'a, Token>>,
}

impl Cursor<'_> {
    fn next(&mut self) -> bool {
        match self.cur {
            Some(ptr) => match ptr.next.get() {
                Some(next_ptr) => {
                    self.cur.replace(next_ptr);
                    true
                }
                None => false,
            },
            None => false,
        }
    }

    fn back(&mut self) -> bool {
        match self.cur {
            Some(ptr) => match ptr.prev.get() {
                Some(next_ptr) => {
                    self.cur.replace(next_ptr);
                    true
                }
                None => false,
            },
            None => false,
        }
    }

    fn access(&self) -> Option<Ref<'_, Token>> {
        self.cur.map(|x| x.val.borrow())
    }

    fn access_mut(&mut self) -> Option<RefMut<'_, Token>> {
        self.cur.map(|x| x.val.borrow_mut())
    }

    fn is_end(&self) -> bool {
        if let Some(ptr) = self.cur {
            ptr.next.get().is_none()
        } else {
            false
        }
    }
}

fn tokenize<'a>(w: &mut Walker, inl: &mut Vec<Inline>, arena: &'a Arena<'a, Node<'a, Token>>) {
    while let Some(char) = w.next() {
        let current_pos = w.position();
        match char as char {
            '\\' => {
                if let Some(next) = w.next() {
                    inl.push(Inline::EscapedChar(next as char))
                }
            }

            cap @ ('_' | '*') => {
                find_delims(w, cap as u8, arena, inl);
            }

            '[' => {
                w.till_not(b'[');
                let end = w.position();

                inl.push(Inline::text(current_pos, end));
                let ptr = inl.last_mut().map(|x| x as *mut Inline);

                let token = Token::new(
                    "[",
                    1,
                    TokenPos::new(current_pos, w.position()),
                    ptr,
                    Ability::Opener,
                );

                arena.to_list(token);
            }

            '!' if w.is_next_char(b'[') => {
                w.advance(1);
                let end = w.position();

                inl.push(Inline::text(current_pos, end));
                let ptr = inl.last_mut().map(|x| x as *mut Inline);

                let token_pos = TokenPos::new(current_pos, w.position());
                let token = Token::new("![", 1, token_pos, ptr, Ability::Opener);

                arena.to_list(token);
            }

            ']' => link_or_image(
                w,
                Cursor {
                    cur: arena.previous(),
                },
                inl,
            ),

            _ => {}
        }
    }
}

#[derive(PartialEq, Eq, Copy, Clone)]
enum Linkable {
    Link = 0,
    Image = 1,
    None = 2,
}

fn determine_whether_link_or_image(dsc: &mut Linkable, val: &str) -> bool {
    match val {
        "![" => {
            *dsc = Linkable::Image;
            true
        }

        "[" => {
            *dsc = Linkable::Link;
            true
        }

        _ => false,
    }
}

fn link_or_image(w: &mut Walker, mut cur: Cursor, inl: &mut Vec<Inline>) {
    let was_found;
    let target: &Node<'_, Token>;
    let mut descriptor = Linkable::None;

    loop {
        if cur
            .access()
            .is_some_and(|node| determine_whether_link_or_image(&mut descriptor, node.char))
        {
            target = cur.cur.unwrap();
            was_found = true;
            break;
        } else {
            cur.back();
        }
    }

    let fail_start = w.position();
    let fail_end = w.position() + 1;

    if !was_found {
        inl.push(Inline::text(fail_start, fail_end));
        return;
    }

    let initial = w.position();

    match w.next() {
        Some(b'(') => {
            let link_start = w.position();

            if w.till(b')').is_some() {
                let name_start = target.val.borrow().pos.start + descriptor as usize;

                let link_end = w.position();
                let link_name = Inline::text(name_start, initial - 1);

                // dbg!(StrRange::new(link_start, link_end).resolve(w.data()));
                // dbg!(StrRange::new(name_start, name_end).resolve(w.data()));

                let mut new = match descriptor {
                    Linkable::Link => Inline::link(StrRange::new(link_start, link_end)),
                    Linkable::Image => Inline::image(StrRange::new(link_start, link_end)),
                    _ => unreachable!(),
                };

                if let Some(vec) = new.expose_inlines() {
                    vec.push(link_name)
                }

                unsafe {
                    if let Some(ptr) = target.val.borrow().node.get() {
                        ptr.replace(new);
                    }
                }

                remove_node(target);
            }
        }

        _ => {
            remove_node(target);
            inl.push(Inline::text(fail_start, fail_end));
        }
    }
}

fn find_delims<'a>(
    w: &mut Walker,
    ch: u8,
    arena: &'a Arena<'a, Node<'a, Token>>,
    inlines: &mut Vec<Inline>,
) {
    let pos = w.position() - 1;

    // after and before default to '\n'
    // because the CommonMark spec specifies
    // that the end and beginning of a line
    // are to be treated as Unicode whitespace
    // '\n' (newline) qualfies to be that.
    let before = if pos == 0 {
        '\n'
    } else {
        let mut before_pos = pos - 1;

        while before_pos > 0 && w.data()[before_pos] >> 6 != 2 {
            before_pos -= 1;
        }

        // dbg!(before_pos);

        // dbg!(w.get(before_pos, pos));

        w.get(before_pos, pos).chars().next_back().unwrap_or('\n')
    };

    let amount = w.till_not(ch) + 1;
    let pos = w.position();

    let after = if w.peek(0).is_none() {
        '\n'
    } else {
        let mut after_pos = pos;

        while after_pos < w.data().len() - 1 {
            after_pos += 1;
        }

        // dbg!(w.get(pos, after_pos));

        w.get(pos, after_pos)
            .chars()
            .next()
            .map_or('\n', |ch| if (ch as usize) > 256 { '\n' } else { ch })
    };

    // dbg!((after, before));

    //                  1. NOT followed by a Unicode whitespace
    let left_flanking = !after.is_whitespace()
        // 2a - either NOT followed by a Unicode punctuation
        && !after.is_punctuation() ||
            // 2b - followed by Unicode punctuation and preceded by either Unicode whitespace or punctuation.
            (after.is_punctuation() && (before.is_whitespace() || before.is_punctuation()));

    //                  1. NOT preceded by Unicode whitespace
    let right_flanking = !before.is_whitespace()
        // 2a - either NOT preceded by Unicode punctuation
        && !before.is_punctuation() ||
            // 2b - preceded by Unicode punctuation and followed by either Unicode whitespace or punctuation.
            (before.is_punctuation() && (after.is_whitespace() || after.is_punctuation()));

    // dbg!((left_flanking, right_flanking));

    let ability = match ch as char {
        '*' => {
            if left_flanking {
                Ability::Opener
            } else if right_flanking {
                Ability::Closer
            } else if right_flanking && left_flanking {
                Ability::Both
            } else {
                Ability::None
            }
        }

        '_' => {
            if left_flanking && !right_flanking || before.is_punctuation() {
                Ability::Opener
            } else if right_flanking && !left_flanking || after.is_punctuation() {
                Ability::Closer
            } else {
                Ability::None
            }
        }

        _ => unreachable!(),
    };

    let delim: &'static str = match ch as char {
        '*' => "*",
        '_' => "_",

        _ => unreachable!(),
    };

    inlines.push(Inline::text(pos, w.position()));

    arena.to_list(Token::new(
        delim,
        amount,
        TokenPos::new(pos, w.position()),
        inlines.last_mut().map(|x| x as *mut Inline),
        ability,
    ));
}

fn process_emphasis(last: Option<*mut Token>, node: *mut Token, bottom: usize) {
    let mut pos = 0;
    let mut openers_bottom: [usize; 2] = [bottom; 2];
}

fn remove_node<T>(node: &Node<T>) {
    let prev = node.prev.get();
    let next = node.next.get();

    next.map(|node| node.prev.replace(prev));
    prev.map(|node| node.next.replace(next));
}

#[cfg(test)]
mod tests {
    use super::DefInlineParser;
    use super::InlineParser;

    #[test]
    fn bold() {
        let data = "![**meow**](veuves)";

        let mut parser = DefInlineParser {};

        let mut inl = parser.parse_inlines(data);
    }
}
