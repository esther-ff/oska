use super::{Block, Parsed, Unparsed};
use crate::md::{
    BlockParser,
    blocks::utils::check_for_possible_new_block,
    chars::{GREATER_THAN, NEWLINE, SPACE},
    walker::Walker,
};

#[derive(Debug)]
pub struct BlkQt<State> {
    level: BlkQtLevel,
    text: Option<Box<Block<State>>>,
    id: usize,
}

impl<State> BlkQt<State> {
    pub fn level(&self) -> usize {
        self.level.0
    }

    pub fn inner(&mut self) -> Option<&mut Block<State>> {
        self.text.as_deref_mut()
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

pub fn make_blockquote<A: Into<Option<Block<Unparsed>>>>(
    text: A,
    id: usize,
    level: usize,
) -> BlkQt<Unparsed> {
    BlkQt {
        level: BlkQtLevel::new(level),
        text: text.into().map(Box::new),
        id,
    }
}

#[derive(Debug)]
pub struct BlkQtLevel(usize);

impl BlkQtLevel {
    fn new(level: usize) -> Self {
        Self(level)
    }
}

pub fn blockquote(parser: &mut impl BlockParser, walker: &mut Walker<'_>) -> Block<Unparsed> {
    let id = parser.get_new_id();
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

    let inner = match parser.block(&mut new_walker) {
        Block::Eof => None,

        val => Some(val),
    };

    Block::make_blockquote(inner, id, level)
}
