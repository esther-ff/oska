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
                if !self.is_list_tight {
                    self.is_list_tight = tight;
                }

                if self.list_origin.is_none() {
                    self.start_bullet_list(input, list_char);
                } else if self.bullet_list_marker != Some(list_char) {
                    self.end_list(input.consumed);
                    return;
                }

                self.insert_list_item(input.consumed + (usize::from(tight) << 1));
                input.consumed += list_start;

                if let Some(empty_line_ix) = input.scan_empty_line() {
                    input.consumed += empty_line_ix;
                    return;
                }

            // Ordered lists
            } else if let Some((list_start, list_char, start_index, tight)) =
                input.scan_ordered_list()
            {
                if !self.is_list_tight {
                    self.is_list_tight = tight;
                }

                if self.list_origin.is_none() {
                    self.start_ordered_list(input, start_index, list_char);
                } else if self.ordered_list_char != Some(list_char) {
                    self.end_list(input.consumed);
                    return;
                }

                let offset = if tight { 2 } else { 0 };
                self.insert_list_item(input.consumed + offset);
                input.consumed += list_start + offset;

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

        if input.eof() {
            return;
        }

        while !input.eof() {
            input.consumed += 1;

            if let Some((level, ix)) = input.scan_setext_heading() {
                self.parse_setext_heading(level, input, old, ix);
                return;
            }

            if input.scan_interrupt_paragraph() {
                break;
            }
        }

        // prevents a '\n\n' paragraph.
        //
        // Safety:
        //
        // The loop above guarantees that the range
        // `old..input.consumed` is in bounds of the data slice.
        unsafe {
            if input.bytes.get_unchecked(old..input.consumed) == b"\n\n" {
                return;
            }
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

        input.consumed += ix + 1; // +1 to skip potential newline

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
        let id = self.tree.right_edge().last().copied();

        if let Some(parent_id) = id
            && let Some(parent) = self.tree.get_mut(parent_id)
        {
            match parent.data.value {
                Value::BulletList { ref mut tight } | Value::OrderedList { ref mut tight, .. } => {
                    *tight = self.is_list_tight;

                    if let Some(id) = self.list_origin.take()
                        && let Some(node) = self.tree.get_mut(id)
                    {
                        node.data.pos.end = end;
                    }

                    self.is_list_tight = false;
                }

                _ => (),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CompileCx;
    use crate::ast::{AstNode, Value};
    use crate::scan::Input;
    use crate::tree::TreeArena;

    macro_rules! test_ast {
        ($src:expr, Limit: $lim:expr, Strict: $strict:expr, $($rules: tt)+) => {{
            type __Rule = (Value, &'static str);
            struct __TestVisitor<'v> {
                src: &'v str,
                rules: Box<[__Rule]>,
                idx: usize,
            }

            impl crate::tree::Visitor for __TestVisitor<'_> {
                fn visit_node(&mut self, val: &AstNode) {
                    if self.idx > $lim {
                        panic!(
                            "too much input, expected {} nodes, found {}",
                            $lim, self.idx
                        );
                    };

                    let txt = val.as_str(self.src);

                    if let Some(cur_rule) = self.rules.get(self.idx) {
                        if *val.value() != cur_rule.0 {
                            panic!(
                                "invalid value ({:#?}) instead of ({:#?})",
                                val.value(),
                                cur_rule.0
                            );
                        }

                        if txt != cur_rule.1 {
                            panic!(
                                "invalid text ({:#?}) instead of ({:#?}) at idx: {}\n at value: {:#?}",
                                txt,
                                cur_rule.1,
                                self.idx,
                                val.value()
                            );
                        }

                        println!(
                            "(order: {:>4}) (type: {:?}) -> ({:5?})",
                            self.idx,
                            val.value(),
                            txt,
                        );
                    } else {
                        if $strict {
                            panic!("more nodes than registered rules in text")
                        } else {
                            println!(
                                "overflow: (order: {}) (type: {:?}) -> ({:#?})",
                                self.idx,
                                val.value(),
                                txt,
                            );
                        }
                    };

                    self.idx += 1
                }
            }

            let input = Input::new($src);

            let c = CompileCx {
                tree: TreeArena::new(),
                bullet_list_marker: None,
                list_origin: None,
                ordered_list_char: None,
                ordered_list_index: None,
                is_list_tight: false,
                inside_macro_invc: false,
            };

            let mut visitor = __TestVisitor {
                src: $src,
                rules: Vec::from([$($rules)+]).into_boxed_slice(),
                idx: 0,
            };

            let tree = c.run(input);

            println!("\nLimit: {} nodes, Strict mode {}!\n", $lim, $strict);
            tree.preorder_visit(&mut visitor)
        }};
    }

    macro_rules! ast_test {
        ($text: expr) => {
            struct __Visitor<'visitor>(&'visitor str, usize);
            impl crate::tree::Visitor for __Visitor<'_> {
                fn visit_node(&mut self, val: &AstNode) {
                    let txt = val.as_str(self.0);

                    println!(
                        "(order: {}) (type: {:#?}) -> ({:#?})",
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
        test_ast!(
            "This is a paragraph!\n\n\
            # This is a heading.", Limit: 4, Strict: true,
            (Value::Paragraph, "This is a paragraph!"),
            (Value::Text, "This is a paragraph!"),
            (Value::Heading { level: core::num::NonZero::new(1).unwrap() }, "# This is a heading."),
            (Value::Text, "This is a heading.")
        );
    }

    #[test]
    fn paragraph() {
        test_ast!(
            "This is a paragraph!",
            Limit: 2, Strict: true,
            (Value::Paragraph, "This is a paragraph!"),
            (Value::Text, "This is a paragraph!")
        );
    }

    #[test]
    fn atx_heading() {
        test_ast!("###### This is a level 6 heading.", Limit: 2, Strict: true,
            (Value::Heading { level: core::num::NonZero::new(6).unwrap() }, "###### This is a level 6 heading."),
            (Value::Text, "This is a level 6 heading.")
        );
    }

    #[test]
    fn ordered_list() {
        const TEST: &str = "1. This is a ordered list >:3\n\
            2. This is again a ordered list\n\
            3. Now the fuss is over...!\n\
            4. We must go to the fire\n";

        test_ast!(
            TEST, Limit: 13, Strict: true,
            (Value::OrderedList { tight: false, start_index: 1 }, TEST),

            (Value::ListItem, "1. This is a ordered list >:3\n"),
            (Value::Paragraph, "This is a ordered list >:3\n"),
            (Value::Text, "This is a ordered list >:3\n"),

            (Value::ListItem, "2. This is again a ordered list\n"),
            (Value::Paragraph, "This is again a ordered list\n"),
            (Value::Text, "This is again a ordered list\n"),

            (Value::ListItem, "3. Now the fuss is over...!\n"),
            (Value::Paragraph, "Now the fuss is over...!\n"),
            (Value::Text, "Now the fuss is over...!\n"),

            (Value::ListItem, "4. We must go to the fire\n"),
            (Value::Paragraph, "We must go to the fire\n"),
            (Value::Text, "We must go to the fire\n")
        );
    }

    #[test]
    fn ordered_list_tight() {
        const TEST: &str = "1. This is a ordered list >:3\n\n\
            2. This is again a ordered list\n\
            3. Now the fuss is over...!\n\
            4. We must go to the fire\n";

        ast_test!(TEST);

        test_ast!(
            TEST, Limit: 13, Strict: true,
            (Value::OrderedList { tight: true, start_index: 1 }, TEST),

            (Value::ListItem, "1. This is a ordered list >:3"),
            (Value::Paragraph, "This is a ordered list >:3"),
            (Value::Text, "This is a ordered list >:3"),

            (Value::ListItem, "2. This is again a ordered list\n"),
            (Value::Paragraph, "This is again a ordered list\n"),
            (Value::Text, "This is again a ordered list\n"),

            (Value::ListItem, "3. Now the fuss is over...!\n"),
            (Value::Paragraph, "Now the fuss is over...!\n"),
            (Value::Text, "Now the fuss is over...!\n"),

            (Value::ListItem, "4. We must go to the fire\n"),
            (Value::Paragraph, "We must go to the fire\n"),
            (Value::Text, "We must go to the fire\n")
        );
    }

    #[test]
    fn bullet_list() {
        const TEST: &str = "- This is a bullet list!\n\
            - Once again a cruel moment\n\
            - Salt water.\n";

        test_ast!(TEST, Limit: 10, Strict: true,
            (Value::BulletList { tight: false }, TEST),

            (Value::ListItem, "- This is a bullet list!\n"),
            (Value::Paragraph, "This is a bullet list!\n"),
            (Value::Text, "This is a bullet list!\n"),

            (Value::ListItem, "- Once again a cruel moment\n"),
            (Value::Paragraph, "Once again a cruel moment\n"),
            (Value::Text, "Once again a cruel moment\n"),

            (Value::ListItem, "- Salt water.\n"),
            (Value::Paragraph, "Salt water.\n"),
            (Value::Text, "Salt water.\n"),
        );
    }

    #[test]
    fn bullet_list_tight() {
        const TEST: &str = "- This is a bullet list!\n\n\
            - Once again a cruel moment\n\
            - Salt water.\n";

        test_ast!(TEST, Limit: 10, Strict: true,
            (Value::BulletList { tight: true }, TEST),

            (Value::ListItem, "- This is a bullet list!"),
            (Value::Paragraph, "This is a bullet list!"),
            (Value::Text, "This is a bullet list!"),

            (Value::ListItem, "- Once again a cruel moment\n"),
            (Value::Paragraph, "Once again a cruel moment\n"),
            (Value::Text, "Once again a cruel moment\n"),

            (Value::ListItem, "- Salt water.\n"),
            (Value::Paragraph, "Salt water.\n"),
            (Value::Text, "Salt water.\n"),
        );
    }

    #[test]
    fn blockquote() {
        test_ast!("> > > Blockquote", Limit: 5, Strict: true,
            (Value::Blockquote, "> > > Blockquote"),
            (Value::Blockquote, "> > Blockquote"),
            (Value::Blockquote, "> Blockquote"),
            (Value::Paragraph, "Blockquote"),
            (Value::Text, "Blockquote")
        );
    }

    #[test]
    fn style_break() {
        test_ast!("------------", Limit: 1, Strict: true,
            (Value::StyleBreak, "------------")
        );
    }

    #[test]
    fn macro_md() {
        test_ast!("<>= macro_test (argument1) (", Limit: 1, Strict: true,
            (Value::Macro { name: "macro_test".into() }, "<>= macro_test (argument1) (")
        );
    }

    #[test]
    fn setext_heading() {
        test_ast!(
            "This is a setext heading!\n\
            =========",
            Limit: 2, Strict: true,
            (Value::Heading { level: core::num::NonZero::new(1).unwrap() },"This is a setext heading!\n\
            ========="),
            (Value::Text, "This is a setext heading!\n")
        );
    }
}
