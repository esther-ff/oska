#![warn(clippy::pedantic)]
#![allow(dead_code)]

use crate::{
    ast::{AstNode, Position, Value},
    scan::{Input, MacroSpan},
    tree::{NodeId, TreeArena},
};

use core::num::NonZero;

pub(crate) struct CompileCx {
    /// Tree structure for the AST
    tree: TreeArena<AstNode>,

    /// Index of the current ordered list.
    ordered_list_index: Option<u64>,

    /// Character that is currently used by the list.
    bullet_list_marker: Option<char>,

    /// Character that comes after the number in an ordered list.
    ordered_list_char: Option<char>,

    /// Points to a `BulletList` or `OrderedList` node.
    list_origin: Option<NodeId>,

    /// Is the list currently processed a tight one
    is_list_tight: bool,

    /// Are we currently in a macro invocation
    inside_macro_invc: bool,
}

impl CompileCx {
    // "compiles" the input to an AST
    fn run(mut self, mut input: Input<'_>) -> TreeArena<AstNode> {
        while !input.eof() {
            self.parse(&mut input);
        }

        self.pop_containers(&input);
        self.end_list(input.consumed);

        while !self.tree.right_edge().is_empty() {
            let _ix = self.tree.go_up();
        }

        self.tree
    }

    // pops any containers on the road to the current node
    fn pop_containers(&mut self, input: &Input<'_>) {
        let i = self.count_containers_to_current_node(input.consumed);
        let len = self.tree.right_edge().len();

        for _ in i..len {
            let _ix = self.tree.go_up();
        }
    }

    // counts containers on way to the current node
    // and assigns them their end positions (`end`)
    fn count_containers_to_current_node(&mut self, end: usize) -> usize {
        let mut amount = 0;

        // dbg!(self.tree.right_edge());
        for ix in 0..self.tree.right_edge().len() {
            let id = self
                .tree
                .right_edge()
                .get(ix)
                .copied()
                .expect("id wasn't present in the tree's spine");

            if let Some(node) = self.tree.get_mut(id)
                && matches!(
                    node.data.value,
                    Value::Blockquote | Value::ListItem | Value::Macro { .. }
                )
            {
                node.data.pos.end = end;

                if matches!(node.data.value, Value::ListItem) {
                    break;
                }
            }
            amount += 1;
        }

        amount
    }

    // parses one block at a time
    fn parse(&mut self, input: &mut Input<'_>) {
        self.pop_containers(input);

        if self.inside_macro_invc && input.scan_macro_end() {
            input.consumed += 1; // skip the ')'
            self.tree.go_up();
            self.inside_macro_invc = false;
        }

        loop {
            // Blockquotes.
            if let Some(blockquote_ix) = input.scan_blockquote() {
                let node = AstNode::new(Value::Blockquote, Position::new(input.consumed, 0), 0);
                input.consumed += blockquote_ix;

                self.tree.attach_node(node);
                self.tree.go_down();

            // Bullet lists
            } else if let Some((list_start, list_char, tight)) = input.scan_bullet_list() {
                self.is_list_tight = tight;

                if self.list_origin.is_none() {
                    self.start_bullet_list(input, list_char);
                } else if self.bullet_list_marker == Some(list_char) {
                    self.end_list(input.consumed);
                    return;
                }

                self.insert_list_item(input.consumed);
                input.consumed += list_start;

                if let Some(empty_line_ix) = input.scan_empty_line() {
                    input.consumed += empty_line_ix;
                    return;
                }

            // Ordered lists
            } else if let Some((list_start, list_char, start_index, tight)) =
                input.scan_ordered_list()
            {
                if self.list_origin.is_none() {
                    self.start_ordered_list(input, start_index, list_char);
                } else if self.ordered_list_char == Some(list_char) {
                    self.end_list(input.consumed);
                    return;
                }

                self.is_list_tight = tight;
                self.insert_list_item(input.consumed);
                input.consumed += list_start;

                if let Some(empty_line_ix) = input.scan_empty_line() {
                    input.consumed += empty_line_ix;
                    return;
                }

            // Macros
            } else if let Some((span, end)) = input.scan_macro()
                && !self.inside_macro_invc
            {
                // for now i forbid nested macros
                // might be funny later
                self.parse_macro(span, end, input);
            } else {
                break;
            }
        }

        if let Some(heading_end) = input.scan_atx_heading() {
            self.end_list(input.consumed);
            self.parse_atx_heading(input, heading_end);
            self.tree.go_up();

            return;
        }

        if let Some(ix) = input.scan_style_break() {
            self.parse_style_break(input, ix);
        }

        self.parse_paragraph(input);
    }

    fn parse_paragraph(&mut self, input: &mut Input<'_>) {
        let old = input.consumed;

        while !input.eof() {
            if let Some((level, ix)) = input.scan_setext_heading() {
                self.parse_setext_heading(level, input, old, ix);
                return;
            }

            if input.scan_interrupt_paragraph() {
                break;
            }

            input.consumed += 1;
        }

        if input.scan_two_newlines() {
            input.consumed += 2;
        }

        if input.consumed == old {
            return;
        }

        let pos = Position::new(old, input.consumed);
        let node = AstNode::new(crate::ast::Value::Paragraph, pos, 0);

        self.tree.attach_node(node);
        self.tree.go_down();
        self.tree.attach_node(AstNode::new(Value::Text, pos, 0));
        self.tree.go_up();
    }

    fn parse_macro(&mut self, span: MacroSpan, end: usize, input: &mut Input<'_>) {
        use core::str::from_utf8_unchecked;

        let (name_start, name_end) = span.name;
        let bytes = &input
            .leftover()
            .get(name_start..name_end - 1)
            .expect("must be present due to earlier scan");

        let node = AstNode::new(
            Value::Macro {
                name: unsafe { String::from(from_utf8_unchecked(bytes)).into_boxed_str() },
            },
            Position::new(input.consumed, input.consumed),
            0,
        );

        input.consumed += end;
        self.tree.attach_node(node);
        self.tree.go_down();
    }

    fn parse_atx_heading(&mut self, input: &mut Input<'_>, heading_end: usize) {
        let node = AstNode::new(
            Value::Heading {
                #[allow(clippy::cast_possible_truncation)]
                level: NonZero::new(heading_end as u8 - 1).expect("value was 0"),
            },
            Position::new(input.consumed, input.consumed + heading_end),
            0,
        );

        self.tree.attach_node(node);
        self.tree.go_down();

        input.consumed += heading_end;
        let mut ix = heading_end;
        let bytes = input.leftover();

        while bytes.get(ix).copied().is_some_and(|byte| byte != b'\n') {
            ix += 1;
        }

        let end = input.consumed + ix;

        if let Some(heading_id) = self.tree.right_edge().last().copied()
            && let Some(heading) = self.tree.get_mut(heading_id)
            && matches!(heading.data.value, Value::Heading { .. })
        {
            heading.data.pos.end = end;
        }

        let node = AstNode::new(Value::Text, Position::new(input.consumed, end), 0);

        input.consumed += ix;

        self.tree.go_up();
        self.tree.attach_node(node);
    }

    fn parse_setext_heading(
        &mut self,
        level: NonZero<u8>,
        input: &mut Input<'_>,
        old_pos: usize,
        ix: usize,
    ) {
        let mut pos = Position::new(old_pos, input.consumed);
        let text = AstNode::new(Value::Text, pos, 0);

        pos.end += ix;
        input.consumed += ix;

        let node = AstNode::new(crate::ast::Value::Heading { level }, pos, 0);

        self.tree.attach_node(node);
        self.tree.go_down();
        self.tree.attach_node(text);
        self.tree.go_up();
    }

    fn parse_style_break(&mut self, input: &mut Input<'_>, ix: usize) {
        let old = input.consumed;
        input.consumed += ix;

        let node = AstNode::new(Value::StyleBreak, Position::new(old, input.consumed), 0);
        self.tree.attach_node(node);
    }

    fn insert_list_item(&mut self, start: usize) {
        self.tree.attach_node(AstNode::new(
            Value::ListItem,
            Position::new(start, start),
            0,
        ));

        self.tree.go_down();
    }

    fn start_ordered_list(&mut self, input: &mut Input<'_>, start_index: u64, list_char: char) {
        let node = AstNode::new(
            Value::OrderedList {
                tight: false,
                start_index,
            },
            Position::new(input.consumed, 0),
            0,
        );

        self.ordered_list_char.replace(list_char);
        self.list_origin.replace(self.tree.attach_node(node));
        self.tree.go_down();
    }

    fn start_bullet_list(&mut self, input: &mut Input<'_>, list_char: char) {
        let node = AstNode::new(
            Value::BulletList { tight: false },
            Position::new(input.consumed, 0),
            0,
        );

        self.bullet_list_marker = Some(list_char);
        self.list_origin.replace(self.tree.attach_node(node));
        self.tree.go_down();
    }

    fn end_list(&mut self, end: usize) {
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
                node.data.pos = Position::new(node.data.pos.start, end);
            }

            self.is_list_tight = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CompileCx;
    use crate::ast::AstNode;
    use crate::scan::Input;
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

            let input = Input::new($text);

            let c = CompileCx {
                tree: TreeArena::new(),
                bullet_list_marker: None,
                list_origin: None,
                ordered_list_char: None,
                ordered_list_index: None,
                is_list_tight: false,
                inside_macro_invc: false,
            };

            let mut visitor = __Visitor($text, 0);

            let tree = c.run(input);

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

    #[test]
    fn style_break() {
        ast_test!("------------");
    }

    #[test]
    fn macro_md() {
        ast_test!("<>= macro_test (argument1) (");
    }

    #[test]
    fn setext_heading() {
        ast_test!(
            "This is a setext heading!\n\
            ========="
        );
    }
}
