use crate::md::{
    Walker,
    chars::{ASTERISK, BACKTICK, NEWLINE, UNDERSCORE},
    inlines::Image,
    walker::StrRange,
};

use super::{
    Block, Document,
    blocks::{Parsed, Unparsed},
    inlines::{EmphasisChar, Inline, Inlines, Text},
};

use core::cell::Cell;

/// A trait representing a parser for inline elements
/// such as emphases or links.
pub trait InlineParser {
    fn parse(&mut self, item: Block<Unparsed>) -> Block<Parsed>;
    fn parse_doc(&mut self, doc: Document<Unparsed>) -> Document<Parsed>;
    fn parse_inlines(&mut self, src: &str) -> Inlines;
}

#[derive(Debug)]
enum Ability {
    Opener,
    Closer,
}

#[derive(Debug)]
struct Token {
    // Delimiter of the token
    char: &'static str,

    // Amount of the delimiter
    amount: usize,

    // Position
    pos: TokenPos,

    // Next and previous tokens
    next: Cell<Option<*mut Token>>,
    prev: Cell<Option<*mut Token>>,

    // Node it is pointing to
    node: Cell<Option<*mut Inline>>,

    ability: Ability,

    closed: bool,
}

#[derive(Debug)]
struct TokenPos {
    start: usize,
    end: usize,
}

impl TokenPos {
    fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    fn tuple(&self) -> (usize, usize) {
        (self.start, self.end)
    }
}

fn remove_node(node: *mut Token) {
    unsafe {
        let prev = (*node).prev.get();
        let next = (*node).next.get();

        next.map(|node| (*node).prev.replace(prev));
        prev.map(|node| (*node).next.replace(next));
    }
}

#[derive(Debug)]
struct TokenContainer {
    col: Vec<Token>,
    last: Option<*mut Token>,
}

impl TokenContainer {
    fn new() -> Self {
        Self {
            col: vec![],
            last: None,
        }
    }

    fn cursor(&mut self) -> Cursor {
        let ptr = self.col.get_mut(0).map(|x| x as *mut Token);
        Cursor {
            list: self,
            cur: ptr,
        }
    }

    fn cursor_last(&mut self) -> Cursor {
        let ptr = self.col.last_mut().map(|x| x as *mut Token);
        Cursor {
            list: self,
            cur: ptr,
        }
    }

    fn last_ptr(&self) -> Option<*mut Token> {
        self.last
    }

    fn push_new(&mut self, token: Token) {
        self.col.push(token);
        let new_last = self.col.last_mut().map(|x| x as *mut Token);
        let len = self.col.len();

        if len >= 2 {
            let old_last = self.col.get_mut(len - 2);
            match old_last {
                None => {}
                Some(inner) => inner.next.set(new_last),
            }
        };

        self.last = self.col.last_mut().map(|x| x as *mut Token);
    }
}

struct Cursor<'a> {
    list: &'a mut TokenContainer,
    cur: Option<*mut Token>,
}

impl Cursor<'_> {
    fn next(&mut self) -> bool {
        match self.cur {
            Some(ptr) => match unsafe { (*ptr).next.get() } {
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
            Some(ptr) => match unsafe { (*ptr).prev.get() } {
                Some(next_ptr) => {
                    self.cur.replace(next_ptr);
                    true
                }
                None => false,
            },
            None => false,
        }
    }

    fn access(&self) -> Option<&Token> {
        self.cur.map(|x| unsafe { &*x })
    }

    fn access_mut(&mut self) -> Option<&mut Token> {
        self.cur.map(|x| unsafe { &mut *x })
    }

    fn is_end(&self) -> bool {
        if let Some(ptr) = self.cur {
            unsafe { (*ptr).next.get().is_none() }
        } else {
            false
        }
    }
}

fn tokenize(tokens: &mut TokenContainer, w: &mut Walker, inl: &mut Vec<Inline>) {
    while let Some(char) = w.next() {
        let current_pos = w.position();
        match char as char {
            '*' => {
                let amount = w.till_not(ASTERISK) + 1;
                let end = w.position();

                inl.push(Inline::text(current_pos, end));
                let ptr = inl.last_mut().map(|x| x as *mut Inline);

                tokens.push_new(Token {
                    char: "*",
                    amount,
                    pos: TokenPos::new(current_pos, end),
                    closed: false,
                    prev: Cell::new(tokens.last_ptr()),
                    next: Cell::new(None),
                    node: Cell::new(ptr),
                    ability: Ability::Opener,
                })
            }

            '_' => {
                let amount = w.till_not(UNDERSCORE) + 1;

                let end = w.position();

                inl.push(Inline::text(current_pos, end));
                let ptr = inl.last_mut().map(|x| x as *mut Inline);
                tokens.push_new(Token {
                    char: "*",
                    amount,
                    pos: TokenPos::new(current_pos, w.position()),
                    closed: false,

                    prev: Cell::new(tokens.last_ptr()),
                    next: Cell::new(None),
                    node: Cell::new(ptr),

                    ability: Ability::Opener,
                })
            }

            '[' => {
                w.till_not(b'[');
                let end = w.position();

                inl.push(Inline::text(current_pos, end));
                let ptr = inl.last_mut().map(|x| x as *mut Inline);
                tokens.push_new(Token {
                    char: "[",
                    amount: 1,
                    pos: TokenPos::new(current_pos, w.position()),
                    closed: false,

                    prev: Cell::new(tokens.last_ptr()),
                    next: Cell::new(None),
                    node: Cell::new(ptr),

                    ability: Ability::Opener,
                })
            }

            '!' if w.is_next_char(b'[') => {
                w.advance(1);
                let end = w.position();

                inl.push(Inline::text(current_pos, end));
                let ptr = inl.last_mut().map(|x| x as *mut Inline);
                tokens.push_new(Token {
                    char: "![",
                    amount: 1,
                    pos: TokenPos::new(current_pos, w.position()),
                    closed: false,

                    prev: Cell::new(tokens.last_ptr()),
                    next: Cell::new(None),
                    node: Cell::new(ptr),

                    ability: Ability::Opener,
                })
            }

            ']' => link_or_image(w, tokens.cursor_last(), inl),

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
    let target: &mut Token;
    let mut descriptor = Linkable::None;

    loop {
        if cur
            .access()
            .is_some_and(|node| determine_whether_link_or_image(&mut descriptor, node.char))
        {
            target = cur.access_mut().expect("infallible, should be present");
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
                let name_start = target.pos.start + descriptor as usize;

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

                if let Some(ptr) = target.node.get() {
                    unsafe {
                        ptr.replace(new);
                    };
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

fn handle_delim() {}

fn process_emphasis(last: Option<*mut Token>, node: *mut Token, bottom: usize) {
    let mut pos = 0;
    let mut openers_bottom: [usize; 2] = [bottom; 2];
}

// Default parser for Inlines
pub struct DefInlineParser {}

impl DefInlineParser {}

impl InlineParser for DefInlineParser {
    fn parse_inlines(&mut self, src: &str) -> Inlines {
        let mut inl = Inlines::new();
        let mut walker = Walker::new(src);
        let mut tokens = TokenContainer::new();

        tokenize(&mut tokens, &mut walker, inl.inner());
        let mut cursor = tokens.cursor();

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

// if delim.char == NEWLINE {
//                 inl.add(Inline::SoftBreak);
//                 continue;
//             }

//             let next_delim = match delim_iter.next() {
//                 Some(val) => val,
//                 None => break,
//             };

//             if delim.char == next_delim.char {
//                 if delim.amnt % 2 == 0 {
//                     let emph = if delim.amnt == usize::from(next_delim.amnt) {
//                         Inline::emph(
//                             true,
//                             EmphasisChar::from_u8(delim.char)
//                                 .expect("this char should always be an asterisk or underscore"),
//                             Inline::text(delim.pos.1, next_delim.pos.0 - 1),
//                         )
//                     } else {
//                         let emph_inner = Inline::emph(
//                             true,
//                             EmphasisChar::from_u8(delim.char)
//                                 .expect("this char should always be an asterisk or underscore"),
//                             Inline::text(delim.pos.1 + 1, next_delim.pos.0),
//                         );

//                         Inline::emph(
//                             false,
//                             EmphasisChar::from_u8(delim.char).unwrap(),
//                             emph_inner,
//                         )
//                     };

//                     inl.add(emph)
//                 } else {
//                     //
//                 }
//             }
// fn parse_one_inline(
//     &mut self,
//     slice: &mut [Delim],
//     old: (usize, usize),
//     inl: &mut Vec<Inline>,
//     inner: bool,
// ) -> bool {
//     let mut iter = slice
//         .iter_mut()
//         .filter(|x| x.binding != Binding::Closed)
//         .enumerate();

//     match iter.next() {
//         None => return false,
//         Some((first_index, val)) => {
//             dbg!(&val);
//             if val.char == '\n' {
//                 val.binding = Binding::Closed;
//                 inl.push(Inline::soft_break());
//                 return true;
//             }

//             while let Some((index, delim)) = iter.next() {
//                 if val.amnt == delim.amnt {
//                     if val.char == delim.char {
//                         let char = match delim.char {
//                             '*' => EmphasisChar::Asterisk,
//                             '_' => EmphasisChar::Underscore,
//                             _ => panic!("invalid"),
//                         };

//                         val.close();
//                         delim.close();

//                         let pos = (val.pos.0 + val.amnt, delim.pos.1 - delim.amnt);

//                         let mut emph = Inline::emph(val.amnt > 1, char);

//                         dbg!(&mut slice[first_index..index]);
//                         self.parse_one_inline(
//                             &mut slice[first_index..index],
//                             pos,
//                             emph.expose_inlines().unwrap(),
//                             true,
//                         );

//                         inl.push(emph);

//                         return true;
//                     }
//                 } else {
//                     delim.close();
//                 }
//             }

//             println!("cannons");
//             val.close();
//             inl.push(Inline::text(val.pos.0, val.pos.1 + 1))
//         }
//     };

//     true
// }
