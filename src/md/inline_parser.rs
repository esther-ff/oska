use crate::md::{
    Document,
    arena::{Arena, Node, NodeRef},
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
        let inl = Inlines::new();
        let mut walker = Walker::new(src);
        let arena = Arena::new();

        tokenize(&mut walker, &inl, &arena);

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

#[derive(Debug)]
enum Ability {
    Opener,
    Closer,
    Both,
    None,
    NotImportant,
}

#[derive(Debug)]
struct Token<'a> {
    // Delimiter of the token
    char: &'static str,

    // Amount of the delimiter
    amount: usize,

    // Position
    pos: TokenPos,

    // Node it is pointing to
    node: &'a Node<'a, Inline>,

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

impl<'a> Token<'a> {
    fn new(
        char: &'static str,
        amount: usize,
        pos: TokenPos,
        node: &'a Node<'a, Inline>,
        ability: Ability,
    ) -> Self {
        Self {
            char,
            amount,
            pos,
            node,
            ability,
            closed: false,
        }
    }

    fn can_close(&self) -> bool {
        matches!(self.ability, Ability::Closer | Ability::Both)
    }

    fn can_open(&self) -> bool {
        matches!(self.ability, Ability::Closer | Ability::Both)
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
    cur: Option<&'a Node<'a, Token<'a>>>,
}

impl<'a> Cursor<'a> {
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

    fn access(&'a self) -> Option<Ref<'a, Token<'a>>> {
        self.cur.map(|x| x.val.borrow())
    }

    fn access_mut(&'a mut self) -> Option<RefMut<'a, Token<'a>>> {
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

fn tokenize<'a>(
    w: &mut Walker,
    inl: &'a Inlines<'a, '_>,
    arena: &'a Arena<'a, Node<'a, Token<'a>>>,
) {
    while let Some(char) = w.next() {
        let current_pos = w.position();
        match char as char {
            '\\' => {
                if let Some(next) = w.next() {
                    inl.add(Inline::EscapedChar(next as char));
                }
            }

            cap @ ('_' | '*') => {
                find_delims(w, cap as u8, arena, inl);
            }

            '[' => {
                w.till_not(b'[');
                let end = w.position();

                let ptr = inl.add(Inline::text(current_pos, end));

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

                let ptr = inl.add(Inline::text(current_pos, end));

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

fn link_or_image(w: &mut Walker, mut cur: Cursor, inl: &mut Inlines) {
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
        inl.add(Inline::text(fail_start, fail_end));
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

                // unsafe {
                //     if let Some(ptr) = target.val.borrow().node.val.borrow_mut() {
                //         *ptr = new
                //     }
                // }

                *target.val.borrow().node.val.borrow_mut() = new;

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
    inlines: &mut Inlines,
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

fn process_emphasis<'a>(arena: &'a Arena<'a, Node<'a, Token>>, bottom: usize) {
    let mut pos = 0;
    let mut openers_bottom: [usize; 12] = [bottom; 12];

    let mut cand = arena.previous();
    let mut closer = None;

    while cand.map_or(false, |x| x.val.borrow().pos.start >= bottom) {
        closer = cand;
        cand = cand.unwrap().prev.get();
    }

    while let Some(rightside) = closer {
        let val = rightside.val.borrow();

        if val.can_close() {
            let index = match val.char {
                "_" => 1,
                "*" => 8 + val.can_open() as usize + val.amount % 3,

                _ => unreachable!("not finished yet"),
            };

            let mut found_opener = false;
            let mut potential_opener = rightside.prev.get();

            while potential_opener
                .map_or(false, |x| x.val.borrow().pos.start >= openers_bottom[index])
            {
                let inner = potential_opener.unwrap().val.borrow();

                if (inner.char == val.char) && inner.can_open() {
                    found_opener = true;
                    let strong_emphasis = inner.amount >= 2 && val.amount >= 2;
                }
            }
        } else {
            todo!()
        }
    }
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
