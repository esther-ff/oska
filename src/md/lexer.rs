use crate::ast;
use crate::unicode::{Utf8, utf8};
use std::error::Error;
use std::fmt;
use std::{io, str};

type LexResult<T> = std::result::Result<T, MdLexerError>;

#[derive(Debug)]
pub enum MdLexerError {
    Io(io::Error),
    Utf8(str::Utf8Error),
}

impl Error for MdLexerError {}
impl fmt::Display for MdLexerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MdLexerError::Io(err) => write!(f, "{}", err),
            MdLexerError::Utf8(err) => write!(f, "{}", err),
        }
    }
}

impl From<str::Utf8Error> for MdLexerError {
    fn from(value: str::Utf8Error) -> Self {
        MdLexerError::Utf8(value)
    }
}

impl From<io::Error> for MdLexerError {
    fn from(value: io::Error) -> Self {
        MdLexerError::Io(value)
    }
}

pub struct MdLexer<'lx> {
    iter: Utf8<'lx>,
    root: Vec<Token<'lx>>,
}

impl<'lx, 'd> MdLexer<'lx>
where
    'lx: 'd,
{
    pub fn new(file: &'lx str) -> LexResult<Self> {
        Ok(Self {
            iter: utf8(file, false),
            root: Vec::with_capacity(16),
        })
    }

    fn is_next(&mut self, target: &str) -> Option<bool> {
        let val = match self.iter.peek() {
            None => return None,
            Some(str) => str,
        };

        dbg!((val, target));

        let is_correct = val == target;

        Some(is_correct)
    }

    pub fn block(&mut self) -> Option<&str> {
        let count = self.iter.pos();

        while let Some(char) = self.iter.next() {
            // if the character we detected is a newline
            let is_newline = char == "\n";
            let (is_next_newline, is_eof) = match self.is_next("\n") {
                None => (false, true),

                Some(val) => (val, false),
            };

            if is_newline && is_next_newline || is_eof {
                let num = self.iter.pos();

                self.iter.eat();
                let part = self.iter.get_from_str(count, num).expect("infallible");

                return Some(part);
            }
        }

        None
    }

    fn paragraph(&'d mut self) {
        let pos = self.iter.pos();
        loop {
            let char = match self.iter.next() {
                Some(char) => char,
                None => break,
            };

            // do something more !

            // if the character we detected is a newline
            let is_newline = char == "\n";
            let (is_next_newline, is_eof) = match self.is_next("\n") {
                None => (false, true),

                Some(val) => (val, false),
            };

            if is_newline && is_next_newline || is_eof {
                let current_pos = self.iter.pos();

                self.iter.eat();
                let part = self
                    .iter
                    .get_from_str(pos, current_pos)
                    .expect("infallible");

                self.root.push(Token::Paragraph(part));

                break;
            }
        }
    }

    fn style_break(&'d mut self, target: &str) -> bool {
        let number_of_chars = self.iter.count_chars(target);
        let is_now_newline = self.is_next("\n").unwrap_or(false);

        if number_of_chars == 2 && is_now_newline {
            self.iter.eat();
            self.root.push(Token::StyleBreak);
            return true;
        }

        false
    }

    fn maybe_bold_italic(&'d mut self) {
        // "target" char
        // the "*" or "_"
        let target = match self.iter.next() {
            Some(char) => char,
            None => return,
        };

        dbg!((target, self.iter.peek()));

        let mut pos = self.iter.pos();

        // if this is equal to the target char
        // we MIGHT be dealing with italics
        let is_doubled = self.is_next(target).unwrap_or(false);

        // function returns a boolean
        // if there was a style break pushed as a node
        // it returns true
        // which terminates this loop
        if self.style_break(target) {
            return;
        }

        // if the target char is doubled
        // eat one character and move our "position " forward
        if is_doubled {
            self.iter.eat();
            pos += 1;
        }

        loop {
            let char = match self.iter.next() {
                Some(char) => char,
                None => break,
            };

            if char == target {
                let curr_pos = self.iter.pos();
                let text = self
                    .iter
                    .get_from_str(pos, curr_pos - 1)
                    .expect("not in bounds");

                let token = if is_doubled && self.is_next(target).unwrap_or(false) {
                    self.iter.eat();
                    ast::Phrasing::Italic(Box::new(Token::Paragraph(text)))
                } else {
                    ast::Phrasing::Bold(Box::new(Token::Paragraph(text)))
                };

                self.root.push(token);
                break;
            }
        }
    }

    // for the above
    // nested italics/bolds could be handled with recursion.

    pub fn lex(&'lx mut self, iter: &mut Utf8<'lx>) {
        loop {
            let char = match iter.peek() {
                None => break,

                Some(char) => char,
            };

            match char {
                "*" | "_" => self.maybe_bold_italic(),

                "\n" => iter.eat(),
                _ => self.paragraph(),
            }
        }

        dbg!(&self.root);
    }

    // for recursion?
    pub fn lex_utf8(&'lx mut self, data: &'lx str) {
        let bytes = data.as_bytes();

        bytes.into_iter().for_each(|ch| match ch {
            b'*' | b'_' => self.maybe_bold_italic(),

            // b'\n' => iter.eat(),
            _ => self.paragraph(),
        });
    }

    pub fn root(&self) -> &[Token<'lx>] {
        &self.root
    }
}

#[derive(Debug)]
pub enum Token<'t> {
    Paragraph(&'t str),
    Bold(Box<Token<'t>>),
    Break,
    Italic(Box<Token<'t>>),
    StyleBreak,
}

// fn get_till<'d, 'b>(lexer: &'d mut MdLexer, target: char) -> (Option<&'b str>, bool)
// where
//     'd: 'b,
// {
//     let count = lexer.cur.cur_cursor();

//     loop {
//         match lexer.go() {
//             None => return (None, true),

//             Some(letter) => {
//                 if letter == target {
//                     let num = lexer.cur.cur_cursor();
//                     let part = lexer.stream.get(count..num - 1);
//                     return (part, false);
//                 }
//             }
//         }
//     }
// }

// fn get_till2<'d, 'b>(lexer: &'d mut MdLexer, target: char) -> (Option<&'b str>, bool)
// where
//     'd: 'b,
// {
//     let cur_location = lexer.iter.pos();
//     loop {
//         let val = match lexer.iter.next() {
//             None => return (None, true),

//             Some(val) => val,
//         };

//         if val == target {
//             let (is_doubled, is_eof) = match lexer.is_next(lexer, target) {
//                 None => (false, true),
//                 Some(val) => (val, false),
//             };

//             if is_doubled {
//                 let num = lexer.iter.pos();
//                 let part = lexer.iter.get_from_str(cur_location, num - 1);
//                 return (part, is_eof);
//             }
//         }
//     }
// }
