use super::{Block, Parsed, Unparsed};
use crate::md::chars::{GREATER_THAN, NEWLINE};
use crate::md::walker::Walker;

#[derive(Debug)]
pub struct BlkQt {
    level: BlkQtLevel,
    text: Option<Box<Block<Unparsed>>>,
    id: usize,
}

#[derive(Debug)]
pub struct BlkQtLevel(usize);

impl BlkQtLevel {
    fn new(level: usize) -> Self {
        Self(level)
    }
}

pub fn blockquote(&mut self, walker: &mut Walker<'_>) -> Block<Unparsed> {
    let id = self.get_new_id();
    let level = walker.till_not(GREATER_THAN);
    let initial = walker.position();

    while let Some(char) = walker.next() {
        match char {
            NEWLINE => {
                if check_for_possible_new_block(walker) {
                    break;
                }
            }

            GREATER_THAN => {
                let amnt_of = walker.till_not(GREATER_THAN);

                if amnt_of > level {
                    walker.retreat(amnt_of + 1);
                    break;
                }

                let advance = usize::from(walker.is_next_char(SPACE));
                walker.advance(advance);
            }

            _ => {}
        }
    }
    let mut new_walker = walker.walker_from_initial(initial);

    let inner = match self.block(&mut new_walker) {
        Block::Eof => None,

        val => Some(val),
    };

    Block::make_blockquote(inner, id, level)
}
