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
    let space = walker.is_next_char(SPACE) as usize;

    while let Some(char) = walker.next() {
        match char {
            NEWLINE => {
                if check_for_possible_new_block(walker) {
                    break;
                }

                // Must test for:
                //
                // Amount of the `>` (must be equal)
                // for more than 1 space
                // as in
                //    > text....
                //
                // is still a valid blockquote
                // if !walker.is_next_char(GREATER_THAN) {
                //     break;
                // }
            }

            GREATER_THAN => {
                let amnt_of = walker.till_not(GREATER_THAN);

                if amnt_of != level {
                    walker.retreat(amnt_of + 1);
                    break;
                }

                let advance = usize::from(walker.is_next_char(SPACE));
                walker.advance(advance);
            }

            _ => {}
        }
    }

    let data = walker.string_from_offset(initial + space);
    let mut string = String::with_capacity(data.len());
    let mut iter = walker
        .string_from_offset(initial + space)
        .chars()
        .peekable();

    while let Some(char) = iter.next() {
        if char == '>' {
            if iter.peek().is_some_and(|x| x == &' ') {
                let _space = iter.next();
            }
        } else {
            string.push(char)
        }
    }

    let mut new_walker = Walker::new(&string);

    // dbg!(core::str::from_utf8(new_walker.data()));
    let inner = match parser.block(&mut new_walker) {
        Block::Eof => None,

        val => Some(val),
    };

    dbg!(walker.peek(0));

    Block::make_blockquote(inner, id, level)
}
