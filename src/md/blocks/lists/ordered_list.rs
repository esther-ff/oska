#[derive(Debug)]
pub struct OrderedList {
    tight: bool,
    start_number: usize,
    items: Vec<ListItem>,
    id: usize,
}

struct OListConstructor {
    items: Vec<ListItem>,
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
        let list_item = ListItem {
            item: Box::new(item),
            number,
        };

        self.items.push(list_item);
    }

    pub fn finish(self, id: usize, tight: bool) -> Block<Unparsed> {
        Block::make_ordered_list(self.cache, self.items, tight, id)
    }
}

pub fn ordered_list(&mut self, start: usize, walker: &mut Walker<'_>) -> Block<Unparsed> {
    if !is_ordered_list_indicator(walker) {
        walker.retreat(1);

        return self.paragraph(walker);
    }

    walker.advance(1);

    let initial = walker.position();
    while let Some(char) = walker.next() {
        if char == NEWLINE && check_for_possible_new_block(walker) {
            break;
        }

        if char == NEWLINE && walker.is_next_pred(|x| x.is_ascii_digit()) {
            walker.advance(1);
            if is_ordered_list_indicator(walker) {
                break;
            }
        }
    }

    let mut new_walker = walker.walker_from_initial(initial);
    let block = self.block(&mut new_walker);

    let mut construct = OListConstructor::new(start - 1);
    let mut tight = true;
    construct.push_item(block);

    walker.advance(1);

    self.ordered_list_inner(walker, &mut construct, &mut tight);

    construct.finish(self.get_new_id(), tight)
}

fn ordered_list_inner(
    &mut self,
    walker: &mut Walker<'_>,
    accum: &mut OListConstructor,
    tightness: &mut bool,
) {
    if !is_ordered_list_indicator(walker) {
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

            walker.advance(1);
            if is_ordered_list_indicator(walker) {
                break;
            }

            walker.retreat(1);
        }
    }

    let mut new_walker = walker.walker_from_initial(initial + 1);
    let block = self.block(&mut new_walker);
    accum.push_item(block);

    walker.advance(1);

    self.ordered_list_inner(walker, accum, tightness);
}
