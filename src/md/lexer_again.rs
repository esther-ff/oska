use core::str;

pub struct Lexer {
    chars: Box<[u8]>,
    cur: usize,
    root: Vec<Token>,
}

const NEWLINE: u8 = b'\n';
const ASTERISK: u8 = b'*';
const UNDERSCORE: u8 = b'_';
const LINE: u8 = b'-';
// const SPACE: u8 = b' ';
const BACKTICK: u8 = b'`';

#[derive(Debug)]
enum Delim {
    Asterisk,
    Underscore,
    Backtick,
    Line,

    BraceLeft,
    BraceRight,

    ExBraceLeft,
    ExBraceRight,
}

#[derive(Debug)]
struct DelimQuery {
    delim: Delim,
    amnt: usize,
    start: usize,
    end: usize,
    str: String,
}

macro_rules! dbg_char {
    ($ch: expr) => {
        let _ = dbg!(core::str::from_utf8(&[$ch.peek().unwrap()]));
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
        }
    }

    /// Peek one char forward.
    pub fn peek(&self) -> Option<u8> {
        self.chars.get(self.cur).copied()
    }

    /// Sets the position of the cursor
    /// NOT RELATIVE
    pub fn set_position(&mut self, num: usize) {
        self.cur = num
    }

    /// Advances the cursor by num
    pub fn advance_by(&mut self, num: usize) {
        self.cur += num
    }

    /// Goes back by num
    pub fn go_back_by(&mut self, num: usize) {
        self.cur -= num
    }

    /// Gives the char and goes 1 back
    pub fn back(&mut self) -> Option<u8> {
        let val = self.chars.get(self.cur).copied();
        self.cur -= 1;

        return val;
    }

    /// returns true if the current character and the next one
    /// form a double newline
    pub fn is_double_newline(&mut self, ch: u8) -> bool {
        ch == NEWLINE && self.is_next_target(NEWLINE)
    }

    /// Finds a character or returns None when the paragraph has ended
    /// meaning when it encounters a sequence of `\n\n`
    pub fn till(&mut self, target: u8) -> Option<&str> {
        let start: usize = self.cur;
        let mut count = self.cur;

        macro_rules! make_str {
            () => {
                self.chars
                    .get(start..count)
                    .map(|x| unsafe { str::from_utf8_unchecked(x) })
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
                    self.advance_by(1);
                    return make_str!();
                }
            }
        }

        None
    }

    /// Finds a character or returns a string without the ending character
    /// caution: ignores paragraph boundaries
    pub fn till_or(&mut self, target: u8) -> &str {
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
                    self.advance_by(1);
                    return make_str!();
                }
            }
        }

        make_str!()
    }

    /// Goes through characters until it finds any char
    /// different than the target
    /// TODO: respect paragraphs
    fn till_not(&mut self, target: u8) -> usize {
        let mut count = 0;

        while let Some(val) = self.next() {
            if val == target {
                count += 1;
            } else {
                self.cur -= 1;
                break;
            }
        }

        count
    }

    /// Checks if the next character is the target
    fn is_next_target(&self, t: u8) -> bool {
        self.peek().map_or_else(|| false, |x| x == t)
    }

    /// Checks if the next char fits the predicate
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
                if f(val) {
                    return true;
                } else {
                    self.cur -= 1;
                    false
                }
            }
        }
    }

    /// Checks if the sequence might be a style break
    fn is_style_break(&mut self) -> bool {
        let initial = self.cur;
        let verify = |x| (x == ASTERISK) | (x == UNDERSCORE) | (x == LINE);

        let maybe_style_break = self.is_next_pred(verify) && self.is_next_pred(verify);

        if !maybe_style_break {
            self.cur = initial;
            return false;
        }

        let initial_pos = self.cur;

        while let Some(char) = self.next() {
            match char {
                ASTERISK | UNDERSCORE | LINE => {}

                NEWLINE => {
                    self.go_back_by(1);
                    break;
                }

                _ => {
                    self.set_position(initial_pos);
                    return false;
                }
            }
        }

        true
    }

    fn is_style_break_peek(&mut self) -> bool {
        self.advance_by(1);
        self.is_style_break()
    }

    fn bold_or_italic(&mut self, target: u8, array: &mut Vec<Inline>) {
        debug_assert!(
            target == ASTERISK || target == UNDERSCORE,
            "invalid character provided to `bold_or_italic`"
        );

        self.go_back_by(1);
        dbg_char!(self);
        let mut delim_vec: Vec<DelimQuery> = Vec::new();

        while let Some(ch) = self.next() {
            // check for style break;
            if (ch == ASTERISK) | (ch == UNDERSCORE) | (ch == LINE) {
                let initial = self.cur;
                if self.is_style_break() {
                    self.cur = initial - 1;
                    break;
                }
            }

            match ch {
                ASTERISK => {
                    let start = self.cur - 1;
                    let amnt = self.till_not(ASTERISK);
                    let text = unsafe {
                        str::from_utf8_unchecked(self.chars.get(start..self.cur).unwrap())
                    };

                    let query = DelimQuery {
                        delim: Delim::Asterisk,
                        amnt: amnt + 1,
                        start,
                        end: self.cur,
                        str: String::from(text),
                    };

                    delim_vec.push(query);
                }

                UNDERSCORE => {
                    let start = self.cur - 1;
                    let amnt = self.till_not(ASTERISK);

                    let text = unsafe {
                        str::from_utf8_unchecked(self.chars.get(start..self.cur).unwrap())
                    };

                    let query = DelimQuery {
                        delim: Delim::Underscore,
                        amnt: amnt + 1,
                        start,
                        end: self.cur,
                        str: String::from(text),
                    };

                    delim_vec.push(query);
                }

                _ => {}
            }
        }

        dbg!(delim_vec);
    }

    /// Parses regular text
    fn text(&mut self) -> Inline {
        let first_start_pos = self.cur;

        loop {
            let val = match self.next() {
                Some(v) => v,

                None => {
                    let text = self
                        .chars
                        .get(first_start_pos..self.cur - 1)
                        .map(|x| str::from_utf8(x).unwrap())
                        .unwrap();

                    return Inline::new(InlineType::Text(text.to_string()));
                }
            };

            match val {
                char if (char == ASTERISK) | (char == UNDERSCORE) => {
                    let initial = self.cur;

                    let asterisk_num = self.till(char);

                    let result = self.till(char);

                    match result {
                        Some(_) => {
                            self.cur = initial;

                            let text = self
                                .chars
                                .get(first_start_pos..initial)
                                .map(|x| str::from_utf8(x).unwrap())
                                .unwrap()
                                .to_string();

                            let inline = Inline::new(InlineType::Text(text));
                            return inline;
                        }

                        None => {}
                    }
                }

                NEWLINE => {
                    if self.is_double_newline(NEWLINE) {
                        let text = self
                            .chars
                            .get(first_start_pos..self.cur)
                            .map(|x| str::from_utf8(x).unwrap())
                            .unwrap()
                            .to_string();

                        let inline = Inline::new(InlineType::Text(text));
                        return inline;
                    };
                }

                _ => {}
            }
        }
    }

    fn inline(&mut self, inlines: &mut Vec<Inline>) {
        while let Some(char) = self.next() {
            // dbg!(str::from_utf8(&[char]));
            match char {
                ASTERISK => {
                    self.bold_or_italic(ASTERISK, inlines);
                }

                UNDERSCORE => self.bold_or_italic(UNDERSCORE, inlines),

                LINE => {
                    if self.is_style_break() {
                        break;
                    }
                }

                NEWLINE => {
                    let double_newline = self.is_next_target(NEWLINE);

                    dbg_char!(self);

                    // a double newline
                    // means the end of a paragraph
                    if double_newline {
                        self.advance_by(1);
                        break;
                    }
                }

                _ => inlines.push(self.text()),
            };
        }
        todo!()
    }

    fn paragraph(&mut self) -> Token {
        let mut inlines = Vec::new();

        while let Some(char) = self.next() {
            match char {
                ASTERISK => {
                    let initial = self.cur;
                    if self.is_style_break() {
                        dbg_char!(self);
                        self.cur = initial - 1;
                        break;
                    } else {
                        self.bold_or_italic(ASTERISK, &mut inlines);
                    }
                }

                UNDERSCORE => self.bold_or_italic(UNDERSCORE, &mut inlines),

                LINE => {
                    if self.is_style_break() {
                        break;
                    }
                }

                NEWLINE => {
                    if self.is_next_target(NEWLINE) {
                        self.advance_by(1);
                        break;
                    }
                }

                _ => inlines.push(self.text()),
            };
        }

        Token::Paragraph(inlines)
    }

    fn lex(&mut self) -> Token {
        let char = match self.next() {
            Some(val) => val,
            None => return Token::Eof,
        };

        match char {
            ASTERISK | UNDERSCORE | LINE => {
                // Checks for a style break
                // or bold or italics.
                if dbg!(self.is_style_break()) {
                    Token::StyleBreak
                } else {
                    self.cur -= 1;
                    self.paragraph()
                }
            }

            BACKTICK => {
                todo!();
            }

            NEWLINE => {
                if self.is_double_newline(NEWLINE) {
                    return Token::Breakline;
                } else {
                    self.advance_by(1);
                    self.lex()
                }
            }

            _ => self.paragraph(),
        }
    }

    pub fn start(mut self) {
        let mut root = Vec::new();

        loop {
            let token = self.lex();

            if token.is_eof() {
                break;
            }

            root.push(token);
        }

        println!("{:#?}", root)
    }
}

/// Describes types of Inlines
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
    StyleBreak,
    Paragraph(Vec<Inline>),
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

// TODO:
// Standardize whether to use peek or next
// Okay?
// Yahar.
