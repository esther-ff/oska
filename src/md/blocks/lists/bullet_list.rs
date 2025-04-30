use super::list_item::ListItem;
use crate::md::{
    blocks::{Block, Parsed, Unparsed},
    chars::{ASTERISK, LINE, NEWLINE, PLUS, SPACE},
    utils::{check_for_possible_new_block, is_bullet_list_marker},
    walker::Walker,
};

#[derive(Debug)]
pub struct BulletList {
    tight: bool,
    items: Vec<ListItem>,
    id: usize,
}

fn bullet_list(&mut self, delim: u8, walker: &mut Walker<'_>) -> Block<Unparsed> {
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
    let block = self.block(&mut new_walker);

    let mut tight = true;

    list_items.push(ListItem {
        number: None,
        item: Box::new(block),
    });

    self.bullet_list_inner(walker, &mut list_items, delim, &mut tight);

    Block::make_bullet_list(list_items, tight, self.get_new_id())
}

fn bullet_list_inner(
    &mut self,
    walker: &mut Walker<'_>,
    accum: &mut Vec<ListItem>,
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
    let block = self.block(&mut new_walker);

    accum.push(ListItem {
        number: None,
        item: Box::new(block),
    });

    self.bullet_list_inner(walker, accum, delim, tight);
}
