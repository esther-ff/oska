use super::blocks::code::meta::{CodeMeta, Lang};
use crate::lib::{String, Vec};
use core::fmt::Debug;
use core::num::NonZero;
/// Temporary type alias
/// Inlines will be replaced with an actual
/// dedicated type
type Inlines1 = Vec<String>;

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub(crate) enum AstType {
    Inline,
    Block,
}

#[derive(Debug)]
pub enum Inlines {
    Parsed(Inlines1),
    Unparsed(String),
}

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
    Blockquote {
        level: NonZero<usize>,
    },

    /// A Stand-alone block of unescaped
    /// HTML inside a Markdown document
    /// contains raw HTML.
    HtmlBlock,

    /// A style break.
    /// `</hr>`
    StyleBreak,

    /// A heading
    /// contains Inlines.
    Heading {
        level: NonZero<u8>,
        atx: bool,
    },

    /// A bullet list
    /// contains List items
    /// which in turn contains blocks.
    ///
    /// ```markdown
    /// - This is a bullet list!
    /// ```
    BulletList {
        tight: bool,
    },

    /// An ordered list
    /// contains List items
    /// which in turn contains blocks.
    ///
    /// ```markdown
    /// 1. This is an ordered list
    /// ```
    OrderedList {
        tight: bool,
    },

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
    Emphasis {
        strong: bool,
    },

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

    HardBreak,

    SoftBreak,

    EscapeChar(char),

    /// Text
    ///
    /// This may be present in a parsed
    /// or unparsed form, depending on the usage
    /// and/or stage of the parsing.
    Text(Inlines),
}

struct Position {
    start: usize,
    end: usize,
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
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn from_source<'p, 'd>(&'p self, src: &'d str) -> Option<&'d str>
    where
        'd: 'p,
    {
        src.get(self.start..self.end)
    }
}

pub struct AstNode {
    ast_type: AstType,
    value: Value,
    pos: Position,
    id: usize,
}

impl Debug for AstNode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let figure = |val: AstType| match val {
            AstType::Inline => "Inline",
            AstType::Block => "Block",
        };

        f.debug_struct("AstNode")
            .field("ast_type", &figure(self.ast_type))
            .field("value", &self.value)
            .field("pos", &self.pos)
            .field("id", &self.id)
            .finish()
    }
}

impl AstNode {
    pub fn new(ast_type: AstType, value: Value, pos: Position, id: usize) -> Self {
        Self {
            ast_type,
            value,
            pos,
            id,
        }
    }

    pub fn value_mut(&mut self) -> &mut Value {
        &mut self.value
    }

    pub fn value(&self) -> &Value {
        &self.value
    }

    pub fn is_inline(&self) -> bool {
        matches!(self.ast_type, AstType::Inline)
    }

    pub fn is_block(&self) -> bool {
        matches!(self.ast_type, AstType::Block)
    }
}
