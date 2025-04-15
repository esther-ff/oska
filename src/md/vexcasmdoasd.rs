use core::str;

pub struct Lexer {
    chars: Box<[u8]>,
    cur: usize,
    root: Vec<Token>,
    state: State,
}

enum State {
    Base,
    Italic(char),
    Bold,
    Underline,
    Paragraph,
}

const NEWLINE: u8 = b'\n';
const ASTERISK: u8 = b'*';
const UNDERSCORE: u8 = b'_';
const LINE: u8 = b'-';
const SPACE: u8 = b' ';
const BACKTICK: u8 = b'`';

macro_rules! dbg_char {
    ($ch: expr) => {
        let _ = dbg!(core::str::from_utf8(&[$ch]))
    };
}

macro_rules! to_str {
    ($expr: expr) => {
        unsafe { core::str::from_utf8_unchecked($expr) }
    };
}

impl Iterator for Lexer {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        let val = self.chars.get(self.cur).copied();
        self.cur += 1;

        return val;
    }
}

impl Lexer {
    pub fn new(data: Vec<u8>) -> Lexer {
        // panic rn
        let result = std::str::from_utf8(&data);
        assert!(result.is_ok());

        let boxed = data.into_boxed_slice();

        Self {
            chars: boxed,
            cur: 0,
            root: Vec::with_capacity(256),
            state: State::Base,
        }
    }

    pub fn peek(&self) -> Option<u8> {
        self.chars.get(self.cur).copied()
    }

    pub fn peek2(&self) -> Option<u8> {
        self.chars.get(self.cur + 1).copied()
    }

    pub fn eat(&mut self) {
        self.cur += 1
    }

    pub fn set_position(&mut self, num: usize) {
        self.cur = num
    }

    pub fn advance_by(&mut self, num: usize) {
        self.cur += num
    }

    // goes back
    pub fn back(&mut self) -> Option<u8> {
        let val = self.chars.get(self.cur).copied();
        self.cur -= 1;

        return val;
    }

    // spits out a line
    pub fn line(&mut self) -> &str {
        self.till(NEWLINE)
    }

    pub fn is_double_newline(&mut self, ch: u8) -> bool {
        ch == NEWLINE && self.is_next_target(NEWLINE)
    }

    // goes till it finds a character
    // TODO: LIMIT THIS TO ONLY A PARAGRAPH
    // etc check for double newline!
    //
    pub fn till(&mut self, target: u8) -> &str {
        let start: usize = self.cur;
        let mut count = self.cur;

        macro_rules! make_str {
            () => {
                self.chars
                    .get(start..count)
                    .map(|x| unsafe { str::from_utf8_unchecked(x) })
                    .unwrap()
            };
        }

        while let Some(val) = self.next() {
            if val == target {
                return make_str!();
            } else {
                count += 1;

                if self.is_next_target(target) {
                    return make_str!();
                }

                if self.is_double_newline(val) {
                    self.eat();
                    return make_str!();
                }
            }
        }

        make_str!()
    }

    // Checks if the next char fits the predicate
    fn is_next_target(&self, t: u8) -> bool {
        //dbg!(str::from_utf8(&[t]));
        self.peek().map_or_else(
            || false,
            |x| {
                // dbg!(str::from_utf8(&[x]));
                x == t
            },
        )
    }

    fn is_next_pred<F>(&mut self, f: F) -> bool
    where
        F: FnOnce(u8) -> bool,
    {
        match self.next() {
            None => {
                self.cur -= 1;
                false
            }

            Some(val) => {
                // dbg_char!(val);
                if f(val) {
                    return true;
                } else {
                    self.cur -= 1;
                    false
                }
            }
        }
    }

    // Checks if the sequence might be a style break
    fn is_style_break(&mut self) -> bool {
        let initial_pos = self.cur;

        // function to verify if the character can form a style break.
        let verify = |char| match char {
            ASTERISK => true,
            UNDERSCORE => true,
            LINE => true,

            _ => false,
        };

        let first = self.is_next_pred(verify);
        if !first {
            return false;
        }

        let second = self.is_next_pred(verify);
        if !second {
            self.set_position(self.cur - 1);
            return false;
        };

        let third = self.is_next_pred(verify);
        if !third {
            self.set_position(self.cur - 2);
            return false;
        };

        let mut is_style_break = true;

        while let Some(char) = self.next() {
            match char {
                ASTERISK | UNDERSCORE | LINE => {}

                NEWLINE => {
                    break;
                }

                _ => {
                    self.set_position(initial_pos);
                    is_style_break = false;
                    break;
                }
            }
        }

        is_style_break
    }

    // fn bold_or_italic(&mut self) {
    //     let target = self.next().unwrap();

    //     // check if next char is the same as the other
    //     let is_double = self.is_next_target(target);
    //     if is_double {
    //         match target {
    //             ASTERISK => self.state = State::Bold,
    //             UNDERSCORE => self.state = State::Underline,

    //             _ => unreachable!(),
    //         }
    //         self.eat()
    //     };

    //     let start_pos: usize = self.cur;
    //     let mut count = 0;
    //     let mut is_double_end = false;

    // Algorithm to find outer stuffs like `**`
    // rough overview is
    // if we know the 2 characters are double
    // we can iterate till we find another 2 characters matching our predicate
    // `**test**`
    //        ^^
    // however we must also check if it's truly the end of that "block"
    // to do that we check if the next character is a different one than the "target"
    // `**test**a`
    //          ^
    // if the check returns true, we create our part of text that contains everything inside
    // the outer block.
    //
    // if not double, uses the `till` function.
    //     let para = if is_double {
    //         while let Some(val) = self.next() {
    //             let peeked_next_val = self.peek().map_or(false, |v| v == target);
    //             if val == target && peeked_next_val {
    //                 self.advance_by(1);

    //                 let is_different = self.peek().map_or(false, |v| v != target);

    //                 if is_different {
    //                     self.cur -= 1;
    //                     is_double_end = true;
    //                     break;
    //                 }

    //                 self.cur -= 1;
    //             }

    //             count += 1;
    //         }

    //         str::from_utf8(self.chars.get(start_pos..start_pos + count).unwrap())
    //             .unwrap()
    //             .to_string()
    //     } else {
    //         // match self.till(target) {
    //         //     Some(val) => val.to_owned(),
    //         //     None => return,
    //         // }
    //         self.till(target).to_owned()
    //     };

    //     self.eat();

    //     // determines token type to insert
    //     let token = if is_double && is_double_end {
    //         self.eat();
    //         match target {
    //             UNDERSCORE => Inline::underline(para.to_owned()),
    //             ASTERISK => Inline::bold(para.to_owned()),

    //             _ => unreachable!(),
    //         }
    //     } else {
    //         Inline::italic(para.to_owned())
    //     };

    //     self.root.push(token);
    // }

    fn parse_code_block(&mut self, blk_type: CodeBlockType) {
        use CodeBlockType::*;

        match blk_type {
            SingleLine => {
                // the main loop in `lex` does not advance this
                self.advance_by(4);

                let inside = self.till(NEWLINE);

                let block = CodeBlock {
                    lang: None,
                    data: None,
                    style: blk_type,
                    contents: inside.to_owned(),
                };

                self.root.push(Token::Code(block))
            }

            Multiline => {
                // todo parse this shit
                todo!()
            }
        }
    }

    // fn paragraph(&mut self) {
    //     let initial_pos = self.cur;

    //     while let Some(char) = self.next() {
    //         let is_newline = char == NEWLINE;
    //         let is_next_newline = self.is_next_target(NEWLINE);

    //         // if it's not a double `\n`
    //         // it should be replaced with a space
    //         if is_newline && !is_next_newline {
    //             *self.chars.get_mut(self.cur - 1).unwrap() = SPACE;
    //         }

    //         // if it is a double `\n`
    //         // it becomes a `BreakLine`
    //         if is_newline && is_next_newline {
    //             self.root.push(Token::Breakline);
    //             break;
    //         }
    //     }

    //     let current_pos = self.cur;

    //     //self.eat();

    //     let part = self
    //         .chars
    //         .get(initial_pos..current_pos - 1)
    //         .expect("infallible");

    //     let actual = unsafe { str::from_utf8_unchecked(part) }.to_owned();

    //     self.root.push(Token::Paragraph(actual));
    // }

    fn bold_or_italic(&mut self, target: u8) -> Inline {
        debug_assert!(
            target == ASTERISK || target == UNDERSCORE,
            "invalid character provided to `bold_or_italic`"
        );

        // we know we have atleast 1 target char consumed
        let initial = self.cur;
        let double = self.is_next_target(target);

        // we can assume that the intended thing here
        // are doubled characters like: `**`
        let mut node = if double {
            let style = match target {
                ASTERISK => InlineType::Bold,
                UNDERSCORE => InlineType::Underline,

                _ => unreachable!(),
            };

            Inline::new(style)
        } else {
            Inline::new(InlineType::Italic)
        };

        let mut double_end = false;
        let start_pos = self.cur;
        let mut count = 0;

        if double {
            while let Some(val) = self.next() {
                let peeked_next_val = self.peek().map_or(false, |v| v == target);
                if val == target && peeked_next_val {
                    self.advance_by(1);

                    let is_different = self.peek().map_or(false, |v| v != target);

                    if is_different {
                        self.cur -= 1;
                        double_end = true;
                        break;
                    }

                    self.cur -= 1;
                }

                count += 1;
            }

            let text = str::from_utf8(self.chars.get(start_pos..start_pos + count).unwrap())
                .unwrap()
                .to_string();

            let inline_text = Inline::new(InlineType::Text(text));

            node.set_inner(inline_text);
        } else {
            // match self.till(target) {
            //     Some(val) => val.to_owned(),
            //     None => return,
            // }
            let text = self.till(target).to_owned();

            node.set_inner(Inline::new(InlineType::Text(text)));
        };

        if !double_end {
            node.set_style(InlineType::Italic);
        };

        node
    }

    fn text(&mut self) -> Inline {
        todo!();
    }

    fn paragraph(&mut self) -> Token {
        let mut inlines = Vec::new();

        while let Some(char) = self.next() {
            match char {
                ASTERISK => inlines.push(self.bold_or_italic(ASTERISK)),
                UNDERSCORE => inlines.push(self.bold_or_italic(UNDERSCORE)),

                NEWLINE => {
                    let double_newline = self.is_next_target(NEWLINE);

                    if double_newline {
                        self.eat();
                        break;
                    } else {
                        // replace singular newlines with spaces
                        *self.chars.get_mut(self.cur).unwrap() = SPACE
                    }
                }

                _ => inlines.push(self.text()),
            };
        }

        Token::Paragraph(inlines)
    }

    fn _lex(&mut self) -> Token {
        let char = match self.peek() {
            Some(val) => val,
            None => return Token::Eof,
        };

        match char {
            ASTERISK | UNDERSCORE | LINE => {
                // Checks for a style break
                // or bold or italics.
                if self.is_style_break() {
                    Token::StyleBreak
                } else {
                    self.paragraph()
                }
            }

            BACKTICK => {
                // we are at 1 detected backtick
                let initial = self.cur;

                self.till(BACKTICK);

                // our cursor is at the first character after the backtick
                // therefore initial - cur is the amount of backticks
                let amnt_of_backticks = self.cur - initial;

                // less than 3 means a inline element
                // emit a paragraph
                if amnt_of_backticks < 3 {
                    let para = self.till(BACKTICK);
                } else {
                }

                todo!();
            }

            _ => self.paragraph(),
        }
    }

    // fn lex(&mut self) {
    //     loop {
    //         let char = match self.peek() {
    //             Some(val) => val,
    //             None => return,
    //         };

    //         match char {
    //             ASTERISK | UNDERSCORE | LINE => {
    //                 // Checks for a style break
    //                 // or bold or italics.
    //                 if self.is_style_break() {
    //                     self.root.push(Token::StyleBreak);
    //                 } else {
    //                     self.bold_or_italic()
    //                 }
    //             }

    //             SPACE => {
    //                 let is_code_block = match self.chars.get(self.cur..self.cur + 4) {
    //                     None => false,
    //                     Some(val) => val == [32, 32, 32, 32],
    //                 };

    //                 if is_code_block {
    //                     let style = CodeBlockType::SingleLine;

    //                     self.parse_code_block(style);
    //                     continue;
    //                 };

    //                 let is_next_character_space =
    //                     self.peek2().map_or(false, |found| found == SPACE);

    //                 if is_next_character_space {
    //                     self.root.push(Token::Breakline);
    //                 }
    //             }

    //             BACKTICK => {
    //                 // we are at 1 detected backtick
    //                 let initial = self.cur;

    //                 self.till(BACKTICK);

    //                 // our cursor is at the first character after the backtick
    //                 // therefore initial - cur is the amount of backticks
    //                 let amnt_of_backticks = self.cur - initial;

    //                 // less than 3 means a inline element
    //                 // emit a paragraph
    //                 if amnt_of_backticks < 3 {
    //                     let para = self.till(BACKTICK);
    //                 } else {
    //                 }
    //             }

    //             NEWLINE => {
    //                 let is_next_character_newline =
    //                     self.peek2().map_or(false, |found| found == NEWLINE);

    //                 if is_next_character_newline {
    //                     self.root.push(Token::Breakline)
    //                 }

    //                 // if `is_next_character_newline` is false
    //                 // we advance by 1 character
    //                 // if it's true, we advance by 2
    //                 self.advance_by(1 + is_next_character_newline as usize);
    //             }

    //             _ => self.paragraph(),
    //         }
    //     }
    // }

    pub fn start(mut self) {
        loop {
            let token = self._lex();

            if token.is_eof() {
                break;
            }

            self.root.push(token);
        }

        println!("{:#?}", Token::Root(self.root))
    }
}

#[derive(Debug)]
pub enum InlineType {
    Code,
    Bold,
    Italic,
    Underline,
    Text(String),
}

#[derive(Debug)]
pub struct Inline {
    style: InlineType,
    inner: Option<Box<Inline>>,
}

impl Inline {
    fn new(style: InlineType) -> Self {
        Self { inner: None, style }
    }

    fn set_inner(&mut self, val: Inline) {
        self.inner = Some(Box::new(val));
    }

    fn set_style(&mut self, style: InlineType) {
        self.style = style;
    }
}

#[derive(Debug)]
enum CodeBlockType {
    // a single line code block
    // the inner bool value means:
    //
    //    true: it was made with a block of 4 spaces (`    `)
    //    false: it was made with a singular backtick (`)
    SingleLine,

    // a multiline code block
    Multiline,
}

#[derive(Debug)]
struct CodeBlock {
    style: CodeBlockType,
    lang: Option<String>,
    data: Option<String>,
    contents: String,
}

#[derive(Debug)]
pub enum Token {
    Breakline,
    // Underline(String),
    StyleBreak,
    // Bold(String),
    // Italic(String),
    Paragraph(Vec<Inline>),
    Root(Vec<Token>),
    Code(CodeBlock),

    Eof,
}

impl Token {
    pub fn is_eof(&self) -> bool {
        match &self {
            &Token::Eof => true,

            _ => false,
        }
    }
}
