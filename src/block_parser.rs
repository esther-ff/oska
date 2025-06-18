use crate::{
    ast::{AstNode, Position, Value},
    tree::{NodeId, TreeArena},
};
use core::{num::NonZero, str};

struct CompileCx<'a> {
    tree: TreeArena<AstNode>,
    text: &'a str,

    /// Amount of consumed bytes from the input
    /// used for positioning
    consumed: usize,

    /// Beginning of the still-yet-to-be consumed
    /// part of the input
    beginning: usize,

    ordered_list_index: Option<u64>,

    bullet_list_marker: Option<u8>,
    list_origin: Option<NodeId>,
    inside_list: bool,
}

impl<'a> CompileCx<'a> {
    fn run(mut self) -> TreeArena<AstNode> {
        while self.consumed < self.text.len() {
            self.parse(&self.text.as_bytes()[self.consumed..])
        }

        while !self.tree.right_edge().is_empty() {
            let _ = self.tree.go_up();
        }

        self.tree
    }

    fn pop_containers(&mut self) {
        let popping = self.consumed;
        dbg!(popping);
        let i = self.count_containers_to_current_node();
        let len = self.tree.right_edge().len();

        for _ in i..len {
            let _id = self.tree.go_up();
        }
    }

    fn count_containers_to_current_node(&mut self) -> usize {
        let mut amount = 0;

        for ix in 0..self.tree.right_edge().len() {
            let id = self
                .tree
                .right_edge()
                .get(ix)
                .copied()
                .expect("id wasn't present in the tree's spine");

            if let Some(node) = self.tree.get_mut(id)
                && matches!(node.data.value, Value::Blockquote { .. } | Value::ListItem)
            {
                // do something here probably idk.
                node.data.pos.end = self.consumed;

                break;
            }

            amount += 1;
        }

        amount
    }

    fn parse(&mut self, mut bytes: &[u8]) {
        self.pop_containers();

        if let Some((bullet_list_start, bullet_list_char)) = self.scan_bullet_list(bytes) {
            if self.list_origin.is_none() {
                let node = AstNode::new(
                    Value::BulletList { tight: false },
                    Position::new(self.consumed, 0),
                    0,
                );

                self.inside_list = true;

                self.list_origin.replace(self.tree.attach_node(node));
                self.tree.go_down();
            }

            bytes = &bytes[bullet_list_start..];
            self.parse_bullet_list(bytes);
            self.consumed += bullet_list_start;
        }

        if let Some((ordered_list_start, ordered_list_start_index)) = self.scan_ordered_list(bytes)
        {
            if self.list_origin.is_none() {
                let node = AstNode::new(
                    Value::OrderedList { tight: false },
                    Position::new(self.consumed, 0),
                    0,
                );

                self.inside_list = true;

                self.list_origin.replace(self.tree.attach_node(node));
                self.tree.go_down();
            }

            bytes = &bytes[ordered_list_start..];
            self.parse_ordered_list(bytes);
            self.consumed += ordered_list_start;
        }

        println!("meow");

        if let Some(heading_end) = self.scan_atx_heading(bytes) {
            self.end_bullet_list();

            let node = AstNode::new(
                Value::Heading {
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
            if self.interrupt_paragraph(&bytes[ix..]) {
                break;
            } else if bytes[ix] == b'\n' && bytes.get(ix + 1).is_some_and(|x| x == &b'\n') {
                ix += 2;
                break;
            };

            ix += 1;
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

        let para = self.consumed;
        dbg!(para);
    }

    fn interrupt_paragraph(&self, bytes: &[u8]) -> bool {
        self.scan_bullet_list(bytes).is_some() || self.scan_atx_heading(bytes).is_some()
    }

    fn parse_atx_heading(&mut self, bytes: &[u8], heading_end: usize) {
        let mut ix = heading_end;
        while bytes.get(ix).copied().is_some_and(|byte| byte != b'\n') {
            ix += 1;
        }

        let node = AstNode::new(
            Value::Text,
            Position::new(self.consumed, self.consumed + ix),
            0,
        );

        self.consumed += ix;

        self.tree.go_up();
        self.tree.attach_node(node);
    }

    // if it scans an atx heading
    // it will go down one node
    // so the function parsing the text
    // has to go back up again
    fn scan_atx_heading(&self, data: &[u8]) -> Option<usize> {
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

    fn parse_bullet_list(&mut self, _bytes: &[u8]) {
        self.tree.attach_node(AstNode::new(
            Value::ListItem,
            Position::new(self.consumed, self.consumed),
            0,
        ));

        let bullet_list = self.consumed;
        dbg!(bullet_list);

        self.tree.go_down();
    }

    fn end_bullet_list(&mut self) {
        // dbg!(self.tree.cursor());
        // dbg!(
        //     self.tree
        //         .right_edge()
        //         .iter()
        //         .map(|x| (x, self.tree.get(*x).unwrap()))
        //         .collect::<Vec<_>>()
        // );
        if let Some(parent) = self
            .tree
            .right_edge()
            .last()
            .copied()
            .map(|id| self.tree.get(id).expect("id not present"))
            && matches!(parent.data.value, Value::BulletList { .. })
        {
            if let Some(id) = self.list_origin.take()
                && let Some(node) = self.tree.get_mut(id)
            {
                let pos = Position::new(node.data.pos.start, self.consumed);

                node.data.pos = pos;
                self.inside_list = false;
            } else {
                unreachable!("node or nodeid was missing")
            }
        }
    }

    // returns (relative index, delimeter character) for the bullet list
    fn scan_bullet_list(&self, bytes: &[u8]) -> Option<(usize, char)> {
        let list_marker_byte = bytes.get(1).copied()?;

        if bytes.first().copied().is_some_and(|x| x.is_ascii_digit())
            && matches!(list_marker_byte, b')' | b'.')
            && bytes.get(2).copied().is_some_and(|byte| byte == b' ')
        {
            Some((3, list_marker_byte as char))
        } else {
            None
        }
    }

    fn parse_ordered_list(&self, _bytes: &[u8]) {
        todo!()
    }

    // returns (relative index, start number of list)
    fn scan_ordered_list(&self, bytes: &[u8]) -> Option<(usize, u64)> {
        let mut number_length = 0;

        let mut ix = 0;
        for (i, byte) in bytes.iter().enumerate() {
            if !byte.is_ascii_digit() {
                break;
            }

            ix = i
        }

        if ix == 0
            || ix >= 9
            || bytes
                .get(ix + 1)
                .copied()
                .is_none_or(|bytechar| bytechar != b'.')
            || bytes
                .get(ix + 2)
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

        ix += 3;

        Some((ix, start_num))
    }
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
                ordered_list_index: None,
                inside_list: false,
            };

            let mut visitor = __Visitor($text, 0);

            let tree = c.run();

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
    fn bullet_list() {
        ast_test!(
            "1. This is a bullet list :3\n\
            2. This is again a bullet list >:3\n\
            3. Now the fuss is over...!\n\
            4. We must go to the fire\n\
            # A grand heading"
        );
    }
}
