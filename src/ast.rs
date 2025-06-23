use crate::lib::{Box, String, ToString};
use core::fmt::Debug;
use core::num::NonZero;

#[derive(Debug)]
pub struct CodeMeta {
    lang: Lang,
    info: Option<String>,
}

impl CodeMeta {
    pub fn new<A: Into<Option<String>>>(lang: Lang, info: A) -> Self {
        Self {
            info: info.into(),
            lang,
        }
    }

    pub fn lang(&self) -> &Lang {
        &self.lang
    }

    pub fn info(&self) -> Option<&String> {
        self.info.as_ref()
    }
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Clone)]
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

/// Value of a Markdown AST node.
#[derive(Debug)]
pub enum Value {
    // Block elements
    /// The root of the document
    /// contains Blocks.
    Root,

    /// Paragraph
    /// contains Inlines.
    Paragraph,

    /// Blockquote
    /// contains Blocks.
    Blockquote,

    /// A stand-alone block of unescaped
    /// HTML inside a Markdown document
    /// contains raw HTML.
    HtmlBlock,

    /// A style break.
    /// `</hr>`
    StyleBreak,

    /// A heading
    /// contains Inlines.
    Heading { level: NonZero<u8>, atx: bool },

    /// A bullet list
    /// contains List items
    /// which in turn contains blocks.
    ///
    /// ```markdown
    /// - This is a bullet list!
    /// ```
    BulletList { tight: bool },

    /// An ordered list
    /// contains List items
    /// which in turn contains blocks.
    ///
    /// ```markdown
    /// 1. This is an ordered list
    /// ```
    OrderedList { tight: bool, start_index: u64 },

    /// List item
    ListItem,

    /// Code
    /// contains Text
    /// however the inline content
    /// is NOT parsed.
    Code {
        lang: Option<Lang>,
        meta: Option<CodeMeta>,
    },

    // Inline elements
    /// An emphasis
    Emphasis { strong: bool },

    /// Link
    ///
    /// ```markdown
    /// [alt](url)
    /// ```
    Link,

    /// Image
    ///
    /// ```markdown
    /// ![alt](url)
    /// ```
    Image,

    /// Strikethrough
    Strikethrough,

    /// Emoji code
    Emoji,

    /// A hard break,
    HardBreak,

    /// A soft break,
    SoftBreak,

    /// Escaped character.
    EscapeChar(char),

    /// Text
    Text,
}

/// Position of the AST node in the
/// source data.
#[derive(Clone, Copy)]
pub struct Position {
    pub start: usize,
    pub end: usize,
}

impl Debug for Position {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "Position: {{ start: {}, end: {} }}",
            self.start, self.end
        )
    }
}

impl Position {
    pub const ZERO_ZERO: Position = Position { start: 0, end: 0 };
    /// Creates a new `Position`
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Cuts out a subslice representing the position.
    ///
    /// The given `src` should be the original data from which
    /// this `Position` was created.
    #[inline]
    pub fn view_substring<'p, 'd>(&'p self, src: &'d str) -> Option<&'d str>
    where
        'd: 'p,
    {
        src.get(self.start..self.end)
    }
}

/// AST Node of a Markdown document
pub struct AstNode {
    pub value: Value,
    pub pos: Position,
    id: usize,
}

impl Debug for AstNode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("AstNode")
            .field("value", &self.value)
            .field("pos", &self.pos)
            .field("id", &self.id)
            .finish()
    }
}

impl AstNode {
    pub fn new(value: Value, pos: Position, id: usize) -> Self {
        Self { value, pos, id }
    }

    pub fn as_str<'a, 'b>(&'b self, data: &'a str) -> &'a str
    where
        'a: 'b,
    {
        self.pos
            .view_substring(data)
            .unwrap_or_else(|| unreachable!("the range was out-of-bounds in `as_str`"))
    }

    pub fn value_mut(&mut self) -> &mut Value {
        &mut self.value
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn is_inline(&self) -> bool {
        !self.is_block()
    }

    pub fn is_block(&self) -> bool {
        matches!(
            self.value,
            Value::Paragraph
                | Value::Blockquote
                | Value::HtmlBlock
                | Value::Heading { .. }
                | Value::ListItem
                | Value::BulletList { .. }
                | Value::OrderedList { .. }
                | Value::Code { .. }
                | Value::HardBreak
                | Value::SoftBreak
        )
    }
}
