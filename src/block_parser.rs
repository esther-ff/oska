#![warn(clippy::pedantic)]
#![allow(dead_code)]

use crate::{
    ast::{AstNode, Position, Value},
    tree::{NodeId, TreeArena},
};
use core::{num::NonZero, str};

pub(crate) struct CompileCx<'a> {
    tree: TreeArena<AstNode>,
    text: &'a str,

    /// Amount of consumed bytes from the input
    /// used for positioning
    consumed: usize,

    /// Beginning of the still-yet-to-be consumed
    /// part of the input
    beginning: usize,

    ordered_list_index: Option<u64>,

    /// Character that is currently used by the list
    bullet_list_marker: Option<char>,

    /// Character that comes after the number in an ordered list.
    ordered_list_char: Option<char>,

    /// Points to a `BulletList` or `OrderedList` node.
    list_origin: Option<NodeId>,

    /// Is the list currently processed a tight one
    is_list_tight: bool,
}

impl CompileCx<'_> {
    fn run(mut self) -> TreeArena<AstNode> {
        while self.consumed < self.text.len() {
            // dbg!((self.consumed, self.text.len()));
            self.parse(&self.text.as_bytes()[self.consumed..]);
        }

        self.pop_containers();
        self.end_list();

        while !self.tree.right_edge().is_empty() {
            let _ix = self.tree.go_up();
        }

        self.tree
    }

    fn pop_containers(&mut self) {
        let i = self.count_containers_to_current_node();
        let len = self.tree.right_edge().len();

        for _ in i..len {
            let _ix = self.tree.go_up();
        }
    }

    fn count_containers_to_current_node(&mut self) -> usize {
        let mut amount = 0;

        dbg!(self.tree.right_edge());
        for ix in 0..self.tree.right_edge().len() {
            let id = self
                .tree
                .right_edge()
                .get(ix)
                .copied()
                .expect("id wasn't present in the tree's spine");

            dbg!(self.tree.get(id));
            if let Some(node) = self.tree.get_mut(id)
                && matches!(node.data.value, Value::Blockquote | Value::ListItem)
            {
                node.data.pos.end = self.consumed;

                // break;
            }
            amount += 1;
        }

        amount
    }

    fn parse(&mut self, mut bytes: &[u8]) {
        self.pop_containers();

        loop {
            dbg!(self.consumed);
            if let Some(blockquote_ix) = scan_blockquote(bytes) {
                let node = AstNode::new(Value::Blockquote, Position::new(self.consumed, 0), 0);
                self.consumed += blockquote_ix;
                bytes = &bytes[blockquote_ix..];

                dbg!(str::from_utf8(&bytes));
                dbg!(scan_blockquote(bytes));
                println!("meoeew");
                self.tree.attach_node(node);
                self.tree.go_down();
            } else if let Some((bullet_list_start, bullet_list_char, tight)) =
                scan_bullet_list(bytes)
            {
                if self.list_origin.is_none() {
                    let node = AstNode::new(
                        Value::BulletList { tight: false },
                        Position::new(self.consumed, 0),
                        0,
                    );

                    self.bullet_list_marker = Some(bullet_list_char);
                    self.list_origin.replace(self.tree.attach_node(node));
                    self.tree.go_down();
                } else if self
                    .bullet_list_marker
                    .is_some_and(|x| x != bullet_list_char)
                {
                    self.end_list();
                    return;
                }

                self.is_list_tight = tight;
                self.insert_list_item();
                self.consumed += bullet_list_start;

                bytes = &bytes[bullet_list_start..];
            } else if let Some((
                ordered_list_start,
                ordered_list_char,
                ordered_list_start_index,
                tight,
            )) = scan_ordered_list(bytes)
            {
                if self.list_origin.is_none() {
                    let node = AstNode::new(
                        Value::OrderedList {
                            tight: false,
                            start_index: ordered_list_start_index,
                        },
                        Position::new(self.consumed, 0),
                        0,
                    );

                    self.ordered_list_char.replace(ordered_list_char);
                    self.list_origin.replace(self.tree.attach_node(node));
                    self.tree.go_down();
                } else if self
                    .ordered_list_char
                    .is_some_and(|x| x != ordered_list_char)
                {
                    self.end_list();
                    return;
                }

                self.is_list_tight = tight;
                self.insert_list_item();
                self.consumed += ordered_list_start;

                bytes = &bytes[ordered_list_start..];
            } else {
                // save location?
                break;
            }
        }

        if let Some(heading_end) = scan_atx_heading(bytes) {
            self.end_list();

            let node = AstNode::new(
                Value::Heading {
                    #[allow(clippy::cast_possible_truncation)]
                    level: NonZero::new(heading_end as u8 - 1).expect("value was 0"),
                    atx: true,
                },
                Position::new(self.consumed, self.consumed + heading_end),
                0,
            );

            self.tree.attach_node(node);
            self.tree.go_down();

            self.consumed += heading_end;
            self.parse_atx_heading(&bytes[heading_end..], heading_end);

            self.tree.go_up();
            return;
        }

        self.parse_paragraph(bytes);
    }

    fn parse_paragraph(&mut self, bytes: &[u8]) {
        let mut ix = 0;

        while ix < bytes.len() {
            if scan_interrupt_paragraph(&bytes[ix..]) {
                break;
            }

            ix += 1;
        }

        if scan_two_newlines(bytes) {
            self.consumed += 2;
        }

        if ix == 0 {
            return;
        }

        let pos = Position::new(self.consumed, self.consumed + ix);
        let node = AstNode::new(crate::ast::Value::Paragraph, pos, 0);

        self.tree.attach_node(node);
        self.tree.go_down();
        self.tree.attach_node(AstNode::new(Value::Text, pos, 0));
        self.tree.go_up();

        self.consumed += ix;
    }

    fn parse_atx_heading(&mut self, bytes: &[u8], heading_end: usize) {
        let mut ix = heading_end;
        while bytes.get(ix).copied().is_some_and(|byte| byte != b'\n') {
            ix += 1;
        }

        let end = self.consumed + ix;

        if let Some(heading_id) = self.tree.right_edge().last().copied()
            && let Some(heading) = self.tree.get_mut(heading_id)
            && matches!(heading.data.value, Value::Heading { atx: true, .. })
        {
            heading.data.pos.end = end;
        }

        let node = AstNode::new(Value::Text, Position::new(self.consumed, end), 0);

        self.consumed += ix;

        self.tree.go_up();
        self.tree.attach_node(node);
    }

    fn insert_list_item(&mut self) {
        self.tree.attach_node(AstNode::new(
            Value::ListItem,
            Position::new(self.consumed, self.consumed),
            0,
        ));

        self.tree.go_down();
    }

    fn end_list(&mut self) {
        // dbg!(
        //     self.tree
        //         .right_edge()
        //         .last()
        //         .copied()
        //         .map(|id| self.tree.get_mut(id).expect("id not present"))
        // );
        if let Some(parent) = self
            .tree
            .right_edge()
            .last()
            .copied()
            .map(|id| self.tree.get_mut(id).expect("id not present"))
            && match parent.data.value {
                Value::BulletList { ref mut tight } | Value::OrderedList { ref mut tight, .. } => {
                    *tight = self.is_list_tight;
                    true
                }

                _ => false,
            }
        {
            if let Some(id) = self.list_origin.take()
                && let Some(node) = self.tree.get_mut(id)
            {
                let pos = Position::new(node.data.pos.start, self.consumed);
                node.data.pos = pos;
            } else {
                unreachable!("node or nodeid was missing")
            }

            self.is_list_tight = false;
        }
    }

    fn parse_ordered_list(&self, _bytes: &[u8]) {
        todo!()
    }
}

// scans for two consecutive newlines
fn scan_two_newlines(bytes: &[u8]) -> bool {
    bytes
        .first()
        .copied()
        .zip(bytes.get(1).copied())
        .is_some_and(|tuple| tuple == (b'\n', b'\n'))
}

// returns (relative index, delimeter character) for the bullet list
fn scan_bullet_list(mut bytes: &[u8]) -> Option<(usize, char, bool)> {
    let tight = scan_two_newlines(bytes);
    let mut relative_index = 2;

    if tight {
        bytes = &bytes[2..];
        relative_index = 4;
    }

    let list_marker_byte = bytes.first().copied().map(|byte| byte as char);

    if list_marker_byte.is_some_and(|x| matches!(x, '-'))
        && bytes
            .get(1)
            .copied()
            .is_some_and(|byte| matches!(byte as char, ' '))
    {
        Some((relative_index, list_marker_byte?, tight))
    } else {
        None
    }
}

// returns (relative index, start number of list)
fn scan_ordered_list(bytes: &[u8]) -> Option<(usize, char, u64, bool)> {
    let tight = scan_two_newlines(bytes);
    let mut ix = if tight { 2 } else { 0 };
    let bytes = &bytes[ix..];

    for (i, byte) in bytes.iter().enumerate() {
        if !byte.is_ascii_digit() {
            if *byte != b'.' && *byte != b')' {
                return None;
            }

            ix += 1;

            break;
        }

        ix = i;
    }

    let marker_char: char = bytes
        .get(ix)
        .copied()
        .map(Into::into)
        .expect("infallible, earlier loop ensures there is something here");

    if ix == 0
        || ix >= 9
        || bytes
            .get(ix + 1)
            .copied()
            .is_none_or(|bytechar| bytechar != b' ')
    {
        return None;
    }

    let start_num = unsafe {
        str::from_utf8_unchecked(&bytes[..ix])
            .parse::<u64>()
            .expect("infallible")
    };

    Some((ix + 2, marker_char, start_num, tight))
}

// Scans the blockquote
// ignores initial whitespace
// returns `Some(index)` if a blockquote is present
// else it returns `None`
fn scan_blockquote(bytes: &[u8]) -> Option<usize> {
    if bytes.first().copied().is_some_and(|char| char == b'>') {
        Some(1 + usize::from(bytes.get(1).copied().is_some_and(|char| char == b' ')))
    } else {
        None
    }
}

// if it scans an atx heading
// it will go down one node
// so the function parsing the text
// has to go back up again
fn scan_atx_heading(data: &[u8]) -> Option<usize> {
    let mut ix = 0;

    while data.get(ix).copied().is_some_and(|byte| byte == b'#') {
        ix += 1;
    }

    if ix == 0
        || ix > 6
        || data.get(ix).is_none()
        || data.get(ix).copied().is_some_and(|x| x != b' ')
    {
        return None;
    }

    // consume all the white space
    while data.get(ix).copied().is_some_and(|byte| byte == b' ') {
        ix += 1;
    }

    Some(ix)
}

// scans if a paragraph has to be interrupted
fn scan_interrupt_paragraph(bytes: &[u8]) -> bool {
    scan_bullet_list(bytes).is_some()
        || scan_ordered_list(bytes).is_some()
        || scan_atx_heading(bytes).is_some()
        || scan_two_newlines(bytes)
}

#[cfg(test)]
mod tests {
    use super::CompileCx;
    use crate::ast::AstNode;
    use crate::tree::TreeArena;

    macro_rules! ast_test {
        ($text: expr) => {
            struct __Visitor<'visitor>(&'visitor str, usize);
            impl crate::tree::Visitor for __Visitor<'_> {
                fn visit_node(&mut self, val: &AstNode) {
                    let txt = val.as_str(self.0);

                    println!(
                        "(order: {}) (type: {:?}) -> ({:#?})",
                        self.1,
                        val.value(),
                        txt,
                    );

                    self.1 += 1
                }
            }

            let c = CompileCx {
                beginning: 0,
                consumed: 0,
                text: $text,
                tree: TreeArena::new(),
                bullet_list_marker: None,
                list_origin: None,
                ordered_list_char: None,
                ordered_list_index: None,
                is_list_tight: false,
            };

            let mut visitor = __Visitor($text, 0);

            let tree = c.run();

            println!("");
            tree.preorder_visit(&mut visitor)
        };
    }

    #[test]
    fn all() {
        ast_test!("This is a paragraph!\n\n# This is a heading.");
    }

    #[test]
    fn paragraph() {
        ast_test!("This is a paragraph!");
    }

    #[test]
    fn heading() {
        ast_test!("###### This is a level 6 heading.");
    }

    #[test]
    fn ordered_list() {
        ast_test!(
            "1. This is a ordered list :3\n\
            2. This is again a ordered list >:3\n\
            3. Now the fuss is over...!\n\
            4. We must go to the fire\n\
            # A grand heading"
        );
    }

    #[test]
    fn bullet_list() {
        ast_test!(
            "- This is a bullet list!\n\
            - Once again a cruel moment\n\
            - Salt water.\n\
            # meow"
        );
    }

    #[test]
    fn blockquote() {
        ast_test!("> > > Blockquote");
    }
}
