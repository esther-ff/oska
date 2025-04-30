use super::{Block, Parsed, Unparsed};
use crate::md::chars::{ASTERISK, LINE, NEWLINE, UNDERSCORE};
use crate::md::walker::Walker;

#[derive(Debug)]
pub struct Break {
    id: usize,
}

pub fn style_break(&mut self, walker: &mut Walker<'_>) -> Option<Block<Unparsed>> {
    let initial = walker.position();

    let pred = |x| (x == ASTERISK) | (x == LINE) | (x == UNDERSCORE);

    if walker.is_next_pred(pred) {
        if walker.peek(2).is_some_and(pred) {
            return None;
        }

        walker.advance(1);
    } else {
        return None;
    }

    while let Some(char) = walker.next() {
        match char {
            ASTERISK | LINE | UNDERSCORE => {}

            NEWLINE => break,

            _ => {
                walker.set_position(initial);
                return None;
            }
        }
    }

    Block::make_style_break(self.get_new_id()).into()
}
