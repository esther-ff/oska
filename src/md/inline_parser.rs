use core::num::NonZero;

use crate::md::{
    Walker,
    chars::{ASTERISK, BACKTICK, UNDERSCORE},
};

use super::{
    Block, Document,
    blocks::{Parsed, Unparsed},
    inlines::{Inline, Inlines},
};

/// A trait representing a parser for inline elements
/// such as emphases or links.
pub trait InlineParser {
    fn parse(&mut self, item: Block<Unparsed>) -> Block<Parsed>;
    fn parse_doc(&mut self, doc: Document<Unparsed>) -> Document<Parsed>;
    fn parse_inlines<'a>(&mut self, src: &'a str) -> Inlines;
}

#[derive(Debug)]
struct Delim {
    char: u8,
    amnt: NonZero<usize>,
    start: usize,
    end: usize,
}

/// Default parser for Inlines
pub struct DefInlineParser {}

impl InlineParser for DefInlineParser {
    fn parse_inlines(&mut self, src: &str) -> Inlines {
        let mut inl = Inlines::new();

        let mut walker = Walker::new(src);
        let mut delims = Vec::new();

        while let Some(char) = walker.next() {
            match char {
                ch @ (ASTERISK | UNDERSCORE | BACKTICK | b':') => {
                    let pos = walker.position();
                    let amnt = walker.till_not(ch);
                    let end = walker.position();

                    delims.push(Delim {
                        char: ch,
                        amnt: NonZero::new(amnt + 1).expect("value was 0"),
                        start: pos - 1,
                        end: end - 1,
                    })
                }

                _ => {}
            }
        }

        let mut delim_iter = delims.iter_mut();

        while let Some(delim) = delim_iter.next() {
            let next_delim = delim_iter.next().unwrap();

            if delim.char == next_delim.char {
                let amnt = usize::from(delim.amnt);
                if amnt % 2 == 0 {
                    if amnt == usize::from(next_delim.amnt) {
                        let emph =
                            Inline::emph(true, '*', Inline::text(delim.end + 1, next_delim.start));

                        inl.add(emph);
                    } else {
                        //
                    }
                } else {
                }
            }
        }

        inl
    }

    fn parse(&mut self, item: Block<Unparsed>) -> Block<Parsed> {
        todo!()
    }

    fn parse_doc(&mut self, doc: Document<Unparsed>) -> Document<Parsed> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::DefInlineParser;
    use super::InlineParser;

    #[test]
    fn bold() {
        let data = "**Sample**";

        let mut parser = DefInlineParser {};

        let mut inl = parser.parse_inlines(data);

        inl.iter_values(data)
            .into_iter()
            .for_each(|x| println!("{:#?}", x));
    }
}
