use super::{Block, Parsed, Unparsed};
use crate::md::chars::{EQUALS, NEWLINE};
use crate::md::walker::Walker;

use crate::md::BlockParser;
use crate::md::blocks::{heading::handle_special_heading, utils::check_for_possible_new_block};

#[derive(Debug)]
pub struct Paragraph {
    text: String,
    id: usize,
}

impl Paragraph {
    pub fn inner(&mut self) -> &mut String {
        &mut self.text
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

pub fn make_paragraph(text: String, id: usize) -> Paragraph {
    Paragraph { text, id }
}

pub fn paragraph(parser: &mut impl BlockParser, walker: &mut Walker<'_>) -> Block<Unparsed> {
    let initial = walker.position();

    while let Some(char) = walker.next() {
        match char {
            ch if (ch == NEWLINE) && walker.is_next_char(NEWLINE) => break,

            NEWLINE => {
                if walker.is_next_char(EQUALS) {
                    if let Some(block) = handle_special_heading(parser, walker, initial) {
                        return block;
                    }
                }

                if check_for_possible_new_block(walker) {
                    break;
                }
            }

            _ => {}
        }
    }

    let mut string = String::with_capacity(walker.position() - initial);
    let mut iter = walker.get(initial, walker.position()).chars();

    if let Some(char) = iter.next() {
        if char != ' ' {
            string.push(char);
        }
    }

    iter.filter(|char| char != &'>')
        .map(|char| if char == '\0' { '\u{FFFD}' } else { char })
        .map(|char| if char == '\n' { ' ' } else { char })
        .for_each(|char| string.push(char));

    if let Some(" ") = string.get(string.len() - 1..) {
        let _space = string.pop();
    }

    dbg!(&string);

    Block::make_paragraph(string, parser.get_new_id())
}
