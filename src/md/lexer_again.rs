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
    pub fn line(&mut self) -> Option<&str> {
        self.till(NEWLINE)
    }

    // goes till it finds a character
    pub fn till(&mut self, target: u8) -> Option<&str> {
        let start: usize = self.cur;

        let mut count = self.cur;

        loop {
            match self.next() {
                None => return None,
                Some(val) => {
                    // dbg!(str::from_utf8(&[val]));

                    if val == target {
                        let string = self
                            .chars
                            .get(start..count)
                            .map(|x| unsafe { str::from_utf8_unchecked(x) });

                        return string;
                    } else {
                        count += 1;

                        if self.is_next_target(target) {
                            let string = self
                                .chars
                                .get(start..count)
                                .map(|x| unsafe { str::from_utf8_unchecked(x) });

                            return string;
                        }
                    }
                }
            }
        }
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

    // matches a repeating sequence of characters
    // giving a const paramater `LEN` of 3 and a character 'A'
    // the function tries to find "AAA".
    fn match_template<const LEN: usize>(&mut self, char: u8) -> bool {
        let temp: [u8; LEN] = [char; LEN];

        let current_pos = self.cur;
        let advance = self.cur + LEN;

        self.chars
            .get(current_pos..advance)
            .map_or(false, |text| to_str!(text) == to_str!(&temp))
    }

    fn bold_or_italic(&mut self) {
        let target = self.next().unwrap();

        // check if next char is the same as the other
        let is_double = self.is_next_target(target);
        if is_double {
            match target {
                ASTERISK => self.state = State::Bold,
                UNDERSCORE => self.state = State::Underline,

                _ => unreachable!(),
            }
            self.eat()
        };

        let start_pos: usize = self.cur;
        let mut count = 0;
        let mut is_double_end = false;

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
        let para = if is_double {
            while let Some(val) = self.next() {
                let peeked_next_val = self.peek().map_or(false, |v| v == target);
                if val == target && peeked_next_val {
                    self.advance_by(1);

                    let is_different = self.peek().map_or(false, |v| v != target);

                    if is_different {
                        self.cur -= 1;
                        is_double_end = true;
                        break;
                    }

                    self.cur -= 1;
                }

                count += 1;
            }

            str::from_utf8(self.chars.get(start_pos..start_pos + count).unwrap())
                .unwrap()
                .to_string()
        } else {
            match self.till(target) {
                Some(val) => val.to_owned(),
                None => return,
            }
        };

        self.eat();

        // determines token type to insert
        let token = if is_double && is_double_end {
            self.eat();
            match target {
                UNDERSCORE => Token::Underline(para.to_owned()),
                ASTERISK => Token::Bold(para.to_owned()),

                _ => unreachable!(),
            }
        } else {
            Token::Italic(para.to_owned())
        };

        self.root.push(token);
    }

    fn paragraph(&mut self) {
        let initial_pos = self.cur;

        while let Some(char) = self.next() {
            let is_newline = char == NEWLINE;
            let is_next_newline = self.is_next_target(NEWLINE);

            // if it's not a double `\n`
            // it should be replaced with a space
            if is_newline && !is_next_newline {
                *self.chars.get_mut(self.cur - 1).unwrap() = SPACE;
            }

            // if it is a double `\n`
            // it becomes a `BreakLine`
            if is_newline && is_next_newline {
                self.root.push(Token::Breakline);
                break;
            }
        }

        let current_pos = self.cur;

        //self.eat();

        let part = self
            .chars
            .get(initial_pos..current_pos - 1)
            .expect("infallible");

        let actual = unsafe { str::from_utf8_unchecked(part) }.to_owned();

        self.root.push(Token::Paragraph(actual));
    }

    fn _print_residual(&self) {
        let string = unsafe { str::from_utf8_unchecked(&self.chars[self.cur..]) };
        println!("residual string: {}", string);
    }

    fn lex(&mut self) {
        loop {
            let char = match self.peek() {
                Some(val) => val,
                None => return,
            };

            match char {
                ASTERISK | UNDERSCORE | LINE => {
                    // Checks for a style break
                    // or bold or italics.
                    if self.is_style_break() {
                        self.root.push(Token::StyleBreak);
                    } else {
                        self.bold_or_italic()
                    }
                }

                SPACE => {
                    let is_next_character_space =
                        self.peek2().map_or(false, |found| found == SPACE);

                    if is_next_character_space {
                        self.root.push(Token::Breakline);
                    }
                }
                NEWLINE => {
                    let is_next_character_newline =
                        self.peek2().map_or(false, |found| found == NEWLINE);

                    if is_next_character_newline {
                        self.root.push(Token::Breakline)
                    }

                    // if `is_next_character_newline` is false
                    // we advance by 1 character
                    // if it's true, we advance by 2
                    self.advance_by(1 + is_next_character_newline as usize);
                }

                _ => self.paragraph(),
            }
        }
    }

    pub fn start(mut self) {
        self.lex();

        println!("{:#?}", Token::Root(self.root))
    }
}

#[derive(Debug)]
pub enum Token {
    Breakline,
    Underline(String),
    StyleBreak,
    Bold(String),
    Italic(String),
    Paragraph(String),
    Root(Vec<Token>),
}
