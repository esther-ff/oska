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

const UNDERLINE_STYLE_BREAK: &str = "___\n";
const ASTERISK_STYLE_BREAK: &str = "***\n";

macro_rules! dbg_char {
    ($ch: expr) => {
        dbg!(core::str::from_utf8(&[$ch]))
    };
}

macro_rules! to_str {
    ($expr: expr) => {
        unsafe { core::str::from_utf8_unchecked($expr) }
    };
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

    pub fn next(&mut self) -> Option<u8> {
        let val = self.chars.get(self.cur).copied();
        self.cur += 1;

        return val;
    }

    pub fn eat(&mut self) {
        self.cur += 1
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

    // i don't know
    fn till_outside<const STEPS: usize>(&mut self, target: u8) -> Option<&str> {
        let pos = self.cur;
        let mut end = self.cur;

        let arr = &[target; STEPS];
        let victim: &str = str::from_utf8(arr).unwrap();

        let mut scratch = [0; STEPS];
        let mut count = 0;

        loop {
            if count == STEPS {
                let buf = unsafe { str::from_utf8_unchecked(&scratch) };

                dbg!(buf);
                let is_next_diff = self.peek().map_or(false, |x| x != target);

                if buf == victim && is_next_diff {
                    let string = unsafe {
                        str::from_utf8_unchecked(self.chars.get(pos..end - STEPS).unwrap())
                    };

                    return Some(string);
                }

                count = 0;
            };

            let char = match self.next() {
                None => return None,
                Some(val) => val,
            };

            scratch[count] = char;

            count += 1;
            end += 1
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

    // Checks if the sequence might be a style break
    fn is_style_break(&mut self) -> bool {
        let pos = self.cur;
        let forward_pos = self.cur + 4;

        let is_three_chars = self.chars.get(pos..forward_pos).map_or(false, |slice| {
            let temp = unsafe { str::from_utf8_unchecked(slice) };
            //dbg!(temp);

            temp == UNDERLINE_STYLE_BREAK || temp == ASTERISK_STYLE_BREAK
        });

        is_three_chars
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
                    self.cur += 1;

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
        let pos = self.cur;
        loop {
            let char = match self.next() {
                Some(char) => char,
                None => break,
            };

            // do something more !

            // if the character we detected is a newline
            let is_newline = char == b'\n';
            let is_next_newline = self.is_next_target(b'\n');

            if is_newline && is_next_newline {
                let current_pos = self.cur;

                self.eat();
                let part = self.chars.get(pos - 1..current_pos).expect("infallible");

                let actual = unsafe { str::from_utf8_unchecked(part) }.to_owned();

                self.root.push(Token::Paragraph(actual));

                // dbg_char!(self.peek().unwrap()); <-- rest of lines somehow get lost?
                // fix!

                break;
            }
        }
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
                ASTERISK | UNDERSCORE => {
                    // Checks for a style break
                    // or bold or italics.
                    if self.is_style_break() {
                        self.root.push(Token::StyleBreak);

                        self.cur += 3;
                    } else {
                        self.bold_or_italic()
                    }
                }

                // this should be changed.
                b'\n' => {
                    self.eat();
                }

                // this too
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
    Underline(String),
    StyleBreak,
    Bold(String),
    Italic(String),
    Paragraph(String),
    Root(Vec<Token>),
}

// GRAND IDEA:
// USE A STATE MACHINE
