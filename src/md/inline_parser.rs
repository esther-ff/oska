use core::num::NonZero;

use crate::md::{
    Walker,
    chars::{ASTERISK, BACKTICK, NEWLINE, UNDERSCORE},
};

use super::{
    Block, Document,
    blocks::{Parsed, Unparsed},
    inlines::{EmphasisChar, Inline, Inlines},
};

/// A trait representing a parser for inline elements
/// such as emphases or links.
pub trait InlineParser {
    fn parse(&mut self, item: Block<Unparsed>) -> Block<Parsed>;
    fn parse_doc(&mut self, doc: Document<Unparsed>) -> Document<Parsed>;
    fn parse_inlines(&mut self, src: &str) -> Inlines;
}

#[derive(Debug)]
struct Delim {
    char: u8,
    amnt: NonZero<usize>,
    start: usize,
    end: usize,
    binding: Binding,
}

#[derive(Debug, Clone, Copy)]
enum Binding {
    None,
    Left,
    Right,
}

impl Binding {
    pub fn rev(&mut self) {
        *self = match *self {
            Binding::Left => Binding::Right,
            Binding::Right => Binding::Left,
            Binding::None => Binding::None,
        }
    }
}

/// Default parser for Inlines
pub struct DefInlineParser {}

impl InlineParser for DefInlineParser {
    fn parse_inlines(&mut self, src: &str) -> Inlines {
        let mut inl = Inlines::new();

        let mut walker = Walker::new(src);
        let mut delims = Vec::new();

        let mut binding = Binding::Left;
        while let Some(char) = walker.next() {
            match char {
                ch @ (ASTERISK | UNDERSCORE) => {
                    let pos = walker.position();
                    let amnt = walker.till_not(ch);
                    let end = walker.position();

                    delims.push(Delim {
                        char: ch,
                        amnt: NonZero::new(amnt + 1).expect("value was 0"),
                        start: pos - 1,
                        end: end - 1,
                        binding,
                    });

                    binding.rev()
                }

                NEWLINE => delims.push(Delim {
                    char: NEWLINE,
                    amnt: NonZero::new(1).unwrap(),
                    start: walker.position(),
                    end: walker.position(),
                    binding: Binding::None,
                }),

                _ => {}
            }
        }

        dbg!(&delims);
        let mut delim_iter = delims.iter_mut();

        while let Some(delim) = delim_iter.next() {
            if delim.char == NEWLINE {
                inl.add(Inline::SoftBreak);
                continue;
            }

            let next_delim = match delim_iter.next() {
                Some(val) => val,
                None => break,
            };

            if delim.char == next_delim.char {
                let amnt = usize::from(delim.amnt);
                if amnt % 2 == 0 {
                    if amnt == usize::from(next_delim.amnt) {
                        let emph = Inline::emph(
                            true,
                            EmphasisChar::from_u8(delim.char)
                                .expect("this char should always be an asterisk or underscore"),
                            Inline::text(delim.end + 1, next_delim.start),
                        );

                        inl.add(emph);
                    } else {
                        let emph_inner = Inline::emph(
                            true,
                            EmphasisChar::from_u8(delim.char)
                                .expect("this char should always be an asterisk or underscore"),
                            Inline::text(delim.end + 1, next_delim.start),
                        );

                        let emph_outer = Inline::emph(
                            false,
                            EmphasisChar::from_u8(delim.char).unwrap(),
                            emph_inner,
                        );

                        inl.add(emph_outer)
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
        let data = "**Sample**\n";

        let mut parser = DefInlineParser {};

        let mut inl = parser.parse_inlines(data);

        inl.iter_values(data)
            .into_iter()
            .for_each(|x| println!("{:#?}", x));
    }
}
