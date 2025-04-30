use crate::md::chars::{EQUALS, HASH, NEWLINE, SPACE};
use crate::md::walker::Walker;
use core::num::NonZero;

use super::{Block, Parsed, Unparsed};

#[derive(Debug)]
pub struct Heading {
    level: Option<HeadingLevel>,
    text: Option<String>,
    id: usize,
}

impl Heading {
    pub fn is_level(&self, cmp: u8) -> bool {
        self.level.is_some_and(|lvl| u8::from(lvl.0) == cmp)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct HeadingLevel(NonZero<u8>);

impl HeadingLevel {
    pub fn new(level: u8) -> Self {
        debug_assert!(level > 0, "level was lower than 1");

        let nonzero = unsafe { NonZero::new_unchecked(level) };

        Self(nonzero)
    }
}

pub fn heading(&mut self, walker: &mut Walker<'_>) -> Option<Block<Unparsed>> {
    let level = if walker.is_next_char(HASH) {
        let temp = walker.till_not(HASH);

        if temp > 5 {
            walker.retreat(temp + 1);
            return None;
        }

        u8::try_from(temp).unwrap_or_else(|_| unreachable!()) + 1
    } else {
        1
    };

    if walker.is_next_char(SPACE) {
        walker.advance(1);
    } else {
        walker.retreat(level as usize);
        return self.paragraph(walker).into();
    }

    let range = String::from(walker.till_inclusive(NEWLINE));
    walker.advance(1);

    Block::make_heading(range, level, self.get_new_id()).into()
}

#[inline]
/// helper function for creating a heading
fn special_heading(
    start: usize,
    end: usize,
    walker: &mut Walker<'_>,
    id: usize,
) -> Block<Unparsed> {
    let string = String::from(walker.get(start, end));
    Block::make_heading(string, 1, id)
}

// helper function for a heading
// made by using equals under some text
// like:
// ```
// My heading
// ==========
// ```
fn handle_special_heading(
    &mut self,
    walker: &mut Walker<'_>,
    initial: usize,
) -> Option<Block<Unparsed>> {
    let mut heading = true;
    let pos = walker.position();
    while let Some(char) = walker.next() {
        match char {
            NEWLINE => {
                // walker.advance(1);
                break;
            }

            EQUALS => {}

            _ => {
                heading = false;
                break;
            }
        }
    }

    if heading {
        Self::special_heading(initial, pos - 1, walker, self.get_new_id()).into()
    } else {
        None
    }
}
