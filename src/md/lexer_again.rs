pub struct Lexer {
    chars: Box<str>,
    cur: usize,
    root: Vec<Token>,
}

const NEWLINE: u8 = b'\n';
const ASTERISK: u8 = b'*';
const UNDERSCORE: u8 = b'_';

impl Lexer {
    pub fn new(data: Vec<u8>) -> Lexer {
        // panic rn
        let result = std::str::from_utf8(&data);
        assert!(result.is_ok());

        let boxed = data.into_boxed_slice();

        Self {
            chars: unsafe { std::str::from_boxed_utf8_unchecked(boxed) },
            cur: 0,
            root: Vec::with_capacity(256),
        }
    }

    pub fn peek(&self) -> Option<u8> {
        self.chars.get(self.cur + 1)
    }

    pub fn next(&mut self) -> Option<u8> {
        let val = self.chars.get(self.cur);
        self.cur += 1;

        return val;
    }

    pub fn eat(&mut self) { self.cur += 1 }

    pub fn back(&mut self) -> Option<u8> {
        let val = self.chars.get(self.cur);
        self.cur -= 1;

        return val;
    }

    pub fn line(&mut self) -> Option<&str> {
        self.till(NEWLINE)
    }

    pub fn till(&mut self, target: u8) -> Option<&str> {
        const START: usize = self.cur;

        let mut count = self.cur;

        loop {
            match self.next() {
                None => return None,
                Some(val) => {
                    if val == target {
                        let string = self.chars.get(START..count);

                        return string;
                    } else {
                        count += 1;
                    }
                }
            }
        }
    }

    fn is_next_target(&self, t: u8) -> bool {
        self.peek().map_or_else(|| false, |x| x == t)
    }

    // call only in the main lexing loop okay
    fn is_style_break(&mut self, target: u8) -> bool {
        if !self.is_next_target(target) {
            return false
        };

        self.eat();

        if !self.is_next_target(target) {
            self.cur -= 1;
            return false;
        }

        self.eat();

        if self.is_next_target(target) {
            self.eat();
            true
        } else {
            self.cur -= 2;
            false
        }
    }

    pub fn start() {
        unimplemented!()
    }

    fn bold_or_italic(&mut self, which: u8) {
        // check if next char is the same as the other
        let is_double = self.is_next_target(which);
        const START_POS: usize = self.cur;

        let para = match self.till(which) {
            Some(val) => val,
            None => panic!("the fuck"),
        };

        let tmp = self.is_next_target(which);
        self.eat();
        is_double_end = self.is_next_target(which) && tmp;

        if is_double && is_double_end {
            let token = match which {
                UNDERSCORE => Token::Underline(&para[1..]),
                ASTERISK => Token::Bold(&para[1..]),
            };

            self.root.push(token)
        } else {
            let token = Token::Italic(para);
        }
    }

    fn paragraph(&mut self) {
        
    }

    fn lex(&mut self) {
        loop {
            let char = match self.next() {
                Some(val) => val,
                None => return,
            };

            match char {
                ch if ch == ASTERISK | ch == UNDERSCORE => {

                if !self.is_style_break(ch) {
                    self.bold_or_italic(ch)                
                } else {
                    self.root.push(Token::StyleBreak)
                }
            },

                _ => self.paragraph(),
            }
        }
    }
}

#[derive(Debug)]
pub enum Token<'t> {
    Underline(&'t str),
    StyleBreak,
    Bold(&'t str),
    Italic(&'t str),
    Paragraph(&'t str),
    Root(Vec<Token<'t>>),
}
