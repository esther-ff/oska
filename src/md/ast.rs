use super::inlines;

enum AstType {
    Inline,
    Block,
}

enum Inlines<'a, 'b> {
    Parsed(inlines::Inlines<'a, 'b>),
    Unparsed(String),
}

enum Value {
    Paragraph { inner: String },
    Blockquote { inner: Box<AstNode>, level: usize },
}

struct Position {
    start: usize,
    end: usize,
}

pub struct AstNode {
    ast_type: AstType,
    value: Value,
    pos: Position,
    data: String,
    id: usize,
}

/// value would mean the general type of the variable
/// then it would be connected by nodes to it's actual value
///
/// blkqt ast node -> block -> block -> block
///
/// each -> is a parent-child connection toa node
