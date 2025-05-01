use super::{
    Block, Document,
    blocks::{Parsed, Unparsed},
};

/// A trait representing a parser for inline elements
/// such as emphases or links.
pub trait InlineParser {
    fn parse(&mut self, item: Block<Unparsed>) -> Block<Parsed>;
    fn parse_doc(&mut self, doc: Document<Unparsed>) -> Block<Parsed>;
}

/// Default parser for Inlines
pub struct DefInlineParser {}

impl InlineParser for DefInlineParser {
    fn parse(&mut self, item: Block<Unparsed>) -> Block<Parsed> {
        todo!()
    }

    fn parse_doc(&mut self, doc: Document<Unparsed>) -> Block<Parsed> {
        todo!()
    }
}
