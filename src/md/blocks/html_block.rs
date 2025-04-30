#[derive(Debug)]
pub struct HtmlBlock {
    inner: String,
    id: usize,
}

use super::Block;
use crate::md::chars::NEWLINE;
use crate::md::html_constants::{HTML_ALLOWED_TAGS, SIMPLE_CONDITIONS};
use crate::md::walker::Walker;

pub fn html_block(&mut self, walker: &mut Walker<'_>) -> Block<Unparsed> {
    let initial = walker.position();

    for (index, cond) in SIMPLE_CONDITIONS.into_iter().enumerate() {
        if walker.find_string(cond[0]) {
            // the `!` must be followed by an ascii alphabetic character
            if index == 7 && !walker.is_next_pred(|x| x.is_ascii_alphabetic()) {
                walker.retreat(1);
                return self.paragraph(walker);
            }

            let first_char_of_end = cond[1]
                .as_bytes()
                .first()
                .expect("infallible, this is a constant");

            'inner: while let Some(char) = walker.next() {
                if char == *first_char_of_end {
                    let result = &cond[1][1..];

                    if walker.find_string(result) {
                        break 'inner;
                    }
                }
            }

            let _ = walker.till_inclusive(NEWLINE);
            let string = String::from(walker.get(initial - 1, walker.position()));

            dbg!(&string);
            return Block::make_html_block(string, self.get_new_id());
        }
    }

    // If the above didn't catch the tag
    // then we only have the last 2 possible conditions left
    let skip = usize::from(walker.is_next_char(b'/'));
    walker.advance(skip);

    for tag in HTML_ALLOWED_TAGS {
        if walker.find_string(tag) {
            while let Some(char) = walker.next() {
                dbg!(char);
                if is_blank_line(walker) {
                    let string = String::from(walker.get(initial - 1, walker.position()));
                    return Block::make_html_block(string, self.get_new_id());
                }
            }

            break;
        }
    }

    while let Some(char) = walker.next() {
        if (char == b'/' && walker.is_next_char(GREATER_THAN))
            || walker.is_next_char(GREATER_THAN)
            || is_blank_line(walker)
            || char == GREATER_THAN
        {
            walker.advance(2);
            break;
        }
    }

    let _ = walker.till(NEWLINE);

    let string = String::from(walker.string_from_offset(initial - 1));

    Block::make_html_block(string, self.get_new_id())
}
