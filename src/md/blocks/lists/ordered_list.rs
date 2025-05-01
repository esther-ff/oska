use crate::md::{
    block_parser::BlockParser,
    blocks::{
        Block, Unparsed,
        lists::list_item::ListItem,
        paragraph::paragraph,
        utils::{check_for_possible_new_block, is_ordered_list_indicator},
    },
    chars::NEWLINE,
    walker::Walker,
};

use core::num::NonZero;

#[derive(Debug)]
pub struct OrderedList<State> {
    tight: bool,
    start_number: usize,
    items: Vec<ListItem<State>>,
    id: usize,
}

impl<State> OrderedList<State> {
    pub fn new(tight: bool, start_number: usize, items: Vec<ListItem<State>>, id: usize) -> Self {
        Self {
            tight,
            start_number,
            items,
            id,
        }
    }

    pub fn is_tight(&self) -> bool {
        self.tight
    }

    pub fn start_number(&self) -> usize {
        self.start_number
    }

    pub fn items_mut(&mut self) -> &mut Vec<ListItem<State>> {
        &mut self.items
    }

    pub fn items(&self) -> &[ListItem<State>] {
        &self.items
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

struct OListConstructor {
    items: Vec<ListItem<Unparsed>>,
    num: usize,
    cache: usize,
}

impl OListConstructor {
    pub fn new(num: usize) -> Self {
        Self {
            items: Vec::new(),
            num,
            cache: num,
        }
    }

    pub fn push_item(&mut self, item: Block<Unparsed>) {
        self.num += 1;

        // Safety:
        //
        // Valid lists start from minimally the number 0
        // and we add 1 at the start
        // which means the number at least will be 1
        // so it qualifies for `NonZero<usize>`
        let number: Option<NonZero<usize>> = unsafe { NonZero::new_unchecked(self.num) }.into();
        self.items.push(ListItem::new(number, item));
    }

    pub fn finish(self, id: usize, tight: bool) -> Block<Unparsed> {
        Block::make_ordered_list(self.cache, self.items, tight, id)
    }
}

pub fn ordered_list(
    parser: &mut impl BlockParser,
    start: usize,
    walker: &mut Walker<'_>,
) -> Block<Unparsed> {
    if !is_ordered_list_indicator(walker) {
        walker.retreat(1);
        return paragraph(parser, walker);
    } else {
        walker.advance(1);
    }

    let initial = walker.position();
    while let Some(char) = walker.next() {
        if char == NEWLINE {
            if check_for_possible_new_block(walker) {
                break;
            } else if walker.is_next_pred(|x| x.is_ascii_digit()) {
                walker.advance(1);
                if is_ordered_list_indicator(walker) {
                    break;
                }
            }
        } else if check_for_possible_new_block(walker) {
            break;
        }
    }

    let mut new_walker = walker.walker_from_initial(initial);
    let block = parser.block(&mut new_walker);
    let mut construct = OListConstructor::new(start - 1);
    let mut tight = true;

    construct.push_item(block);
    walker.advance(1);

    dbg!(walker.peek(0));
    ordered_list_inner(parser, walker, &mut construct, &mut tight);

    dbg!(walker.peek(0));
    construct.finish(parser.get_new_id(), tight)
}

fn ordered_list_inner(
    parser: &mut impl BlockParser,
    walker: &mut Walker<'_>,
    accum: &mut OListConstructor,
    tightness: &mut bool,
) {
    if !is_ordered_list_indicator(walker) {
        dbg!(walker.peek(0));
        println!("MEEEOW");
        walker.retreat(1);
        return;
    }

    let initial = walker.position();

    while let Some(char) = walker.next() {
        if char == NEWLINE {
            if check_for_possible_new_block(walker) {
                break;
            }

            if walker.is_next_char(NEWLINE) {
                *tightness = false;
                walker.advance(1);
            }
        }

        if is_ordered_list_indicator(walker) {
            break;
        }
    }

    let mut new_walker = walker.walker_from_initial(initial + 1);
    accum.push_item(parser.block(&mut new_walker));

    walker.advance(1);

    ordered_list_inner(parser, walker, accum, tightness);
}
