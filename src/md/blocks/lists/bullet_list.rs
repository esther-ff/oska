use super::list_item::ListItem;
use crate::md::{
    block_parser::BlockParser,
    blocks::utils::{check_for_possible_new_block, is_bullet_list_marker},
    blocks::{Block, Parsed, Unparsed},
    chars::{ASTERISK, LINE, NEWLINE, PLUS, SPACE},
    walker::Walker,
};

#[derive(Debug)]
pub struct BulletList<State> {
    tight: bool,
    items: Vec<ListItem<State>>,
    id: usize,
}

impl<State> BulletList<State> {
    pub fn is_tight(&self) -> bool {
        self.tight
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

pub fn make_bullet_list(
    items: Vec<ListItem<Unparsed>>,
    tight: bool,
    id: usize,
) -> BulletList<Unparsed> {
    BulletList { tight, items, id }
}

pub fn bullet_list(
    parser: &mut impl BlockParser,
    delim: u8,
    walker: &mut Walker<'_>,
) -> Block<Unparsed> {
    debug_assert!(
        matches!(delim, PLUS | ASTERISK | LINE),
        "char given to `bullet_list` was not a `+`, a `*` nor a `-`"
    );

    let initial = walker.position();
    while let Some(char) = walker.next() {
        if char == NEWLINE && check_for_possible_new_block(walker) {
            break;
        }

        if char == NEWLINE
            && walker.is_next_pred(is_bullet_list_marker)
            && walker.peek(1) == Some(SPACE)
        {
            break;
        }
    }

    let mut list_items = Vec::new();
    let mut new_walker = walker.walker_from_initial(initial);
    let block = parser.block(&mut new_walker);
    let mut tight = true;

    list_items.push(ListItem::new(None, block));

    bullet_list_inner(parser, walker, &mut list_items, delim, &mut tight);
    Block::make_bullet_list(list_items, tight, parser.get_new_id())
}

fn bullet_list_inner(
    parser: &mut impl BlockParser,
    walker: &mut Walker<'_>,
    accum: &mut Vec<ListItem<Unparsed>>,
    delim: u8,
    tight: &mut bool,
) {
    debug_assert!(
        matches!(delim, PLUS | ASTERISK | LINE),
        "char given to `bullet_list_inner` was not a `+`, a `*` nor a `-`"
    );

    if !walker.is_next_pred(is_bullet_list_marker) && walker.peek(0) != Some(delim) {
        return;
    }

    let initial = walker.position();
    while let Some(char) = walker.next() {
        if char == NEWLINE {
            if check_for_possible_new_block(walker) {
                break;
            }

            if walker.is_next_char(NEWLINE) {
                *tight = false;
                walker.advance(1);
            }

            if walker.peek(0) != Some(delim) && walker.peek(0).is_some_and(is_bullet_list_marker) {
                walker.retreat(1);
                return;
            }

            if walker.is_next_pred(|x| x == delim) && walker.peek(1) == Some(SPACE) {
                break;
            }
        }
    }

    let mut new_walker = walker.walker_from_initial(initial + 1);
    let block = parser.block(&mut new_walker);

    accum.push(ListItem::new(None, block));
    bullet_list_inner(parser, walker, accum, delim, tight);
}
