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
    amnt: usize,
    pos: (usize, usize),
    binding: Binding,
}

impl Delim {
    fn new(char: u8, amnt: usize, start: usize, end: usize, binding: Binding) -> Self {
        Self {
            char,
            amnt,
            pos: (start, end),
            binding,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Binding {
    None,
    Left,
    Right,
    Closed,
}

impl Binding {
    pub fn rev(&mut self) {
        *self = match *self {
            Binding::Left => Binding::Right,
            Binding::Right => Binding::Left,
            Binding::None => panic!("do not reverse none"),
            Binding::Closed => panic!("do not reverse close"),
        }
    }
}

/// Default parser for Inlines
pub struct DefInlineParser {}

fn delimeters(walker: &mut Walker) -> Vec<Delim> {
    let mut delims = Vec::new();
    let mut binding = Binding::Left;
    let mut last_char: [(u8, Binding); 2] =
        [(ASTERISK, Binding::None), (UNDERSCORE, Binding::None)];

    while let Some(char) = walker.next() {
        match char {
            ASTERISK => {
                let start = walker.position();

                if last_char[0].1 == Binding::None {
                    last_char[0].1 = Binding::Left
                }

                delims.push(Delim::new(
                    ASTERISK,
                    walker.till_not(ASTERISK) + 1,
                    start,
                    walker.position(),
                    last_char[0].1,
                ));

                last_char[0].1.rev();
            }

            UNDERSCORE => {
                let start = walker.position();

                if last_char[1].1 == Binding::None {
                    last_char[1].1 = Binding::Left
                }

                delims.push(Delim::new(
                    UNDERSCORE,
                    walker.till_not(UNDERSCORE) + 1,
                    start,
                    walker.position(),
                    last_char[1].1,
                ));

                last_char[1].1.rev();
            }

            NEWLINE => delims.push(Delim::new(
                NEWLINE,
                1,
                walker.position(),
                walker.position(),
                Binding::None,
            )),

            _ => {}
        }
    }

    delims
}

impl DefInlineParser {
    // fn parse_inlines_inner(&mut self, iterations: usize, delims: &mut [Delim]) -> Inline {
    //     dbg!(&delims);

    //     // if iterations == delims.len() - 1 {
    //     //     return;
    //     // }

    //     let mut iter_mut = delims[0..]
    //         .iter_mut()
    //         .filter(|x| x.binding != Binding::Closed);

    //     let elem = iter_mut.next().unwrap();

    //     for element in iter_mut {
    //         if (element.char == elem.char)
    //             && (element.amnt >= elem.amnt)
    //             && element.binding == Binding::Right
    //         {
    //             elem.binding = Binding::Closed;
    //             element.binding = Binding::Closed;

    //             if elem.amnt > 1 {
    //                 let emph = Inline::emph(
    //                     false,
    //                     EmphasisChar::from_u8(elem.char).unwrap(),
    //                     self.parse_inlines_inner(iterations, delims),
    //                 );

    //                 return emph;
    //             } else {
    //                 let emph = Inline::emph(
    //                     false,
    //                     EmphasisChar::from_u8(elem.char).unwrap(),
    //                     self.parse_inlines_inner(iterations, delims),
    //                 );

    //                 return emph;
    //             }
    //         }
    //     }

    //     unreachable!()
    // }

    fn parse_one_inline(&mut self, slice: &mut [Delim], old: (usize, usize)) -> Inline {
        let mut iter = slice
            .iter_mut()
            .filter(|x| x.binding != Binding::Closed)
            .enumerate();

        let (first_index, val) = match iter.next() {
            None => return Inline::text(old.0, old.1),
            Some((f, v)) => (f, v),
        };

        if val.char == NEWLINE {
            val.binding = Binding::Closed;
            return Inline::soft_break();
        }

        while let Some((index, delim)) = iter.next() {
            dbg!((&val, &delim));
            if val.amnt == delim.amnt {
                if val.char == delim.char {
                    let char = EmphasisChar::from_u8(val.char).unwrap();

                    val.binding = Binding::Closed;
                    delim.binding = Binding::Closed;

                    let start = val.pos.1;
                    let end = delim.pos.0 - 1;

                    return Inline::emph(
                        val.amnt > 1,
                        char,
                        self.parse_one_inline(&mut slice[first_index..index], (start, end)),
                    );
                }
            } else {
                delim.binding = Binding::Closed;
                // return Inline::text(delim.pos.0, delim.pos.1);
            }
        }

        dbg!(val.char);
        if val.char != NEWLINE {
            Inline::text(val.pos.0, val.pos.1 + 1)
        } else {
            Inline::text(old.0, old.1 + 1)
        }
    }
}

impl InlineParser for DefInlineParser {
    fn parse_inlines(&mut self, src: &str) -> Inlines {
        let mut inl = Inlines::new();
        let mut walker = Walker::new(src);

        let mut delims = delimeters(&mut walker);

        dbg!(&delims);

        inl.add(self.parse_one_inline(&mut delims, (0, 0)));
        inl.add(self.parse_one_inline(&mut delims, (0, 0)));

        dbg!(&inl);

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
        let data = "**__Sam_ple__**_\n";

        let mut parser = DefInlineParser {};

        let mut inl = parser.parse_inlines(data);

        // inl.iter_values(data)
        //     .into_iter()
        //     .for_each(|x| println!("{:#?}", x));
    }
}

// if delim.char == NEWLINE {
//                 inl.add(Inline::SoftBreak);
//                 continue;
//             }

//             let next_delim = match delim_iter.next() {
//                 Some(val) => val,
//                 None => break,
//             };

//             if delim.char == next_delim.char {
//                 if delim.amnt % 2 == 0 {
//                     let emph = if delim.amnt == usize::from(next_delim.amnt) {
//                         Inline::emph(
//                             true,
//                             EmphasisChar::from_u8(delim.char)
//                                 .expect("this char should always be an asterisk or underscore"),
//                             Inline::text(delim.pos.1, next_delim.pos.0 - 1),
//                         )
//                     } else {
//                         let emph_inner = Inline::emph(
//                             true,
//                             EmphasisChar::from_u8(delim.char)
//                                 .expect("this char should always be an asterisk or underscore"),
//                             Inline::text(delim.pos.1 + 1, next_delim.pos.0),
//                         );

//                         Inline::emph(
//                             false,
//                             EmphasisChar::from_u8(delim.char).unwrap(),
//                             emph_inner,
//                         )
//                     };

//                     inl.add(emph)
//                 } else {
//                     //
//                 }
//             }
