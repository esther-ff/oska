#![warn(clippy::all)]
use super::ast::{AstNode, Position, Value};
use super::tree::{NodeAst, TreeArena, Visitor};
use crate::walker::Walker;
use core::num::NonZero;

enum State {
    Paragraph,
    AtxHeading,
    SetextHeading,
    BulletList,
    OrderedList,
    BlockQuote,
    Code(u8),
    IndentedCode,
}

impl Default for State {
    fn default() -> Self {
        Self::Paragraph
    }
}

pub struct Parser {
    state: State,
    tree: TreeArena<AstNode>,
    id: usize,

    heading_size: u8,
}

impl Parser {
    pub fn new() -> Self {
        Self {
            state: State::default(),
            tree: TreeArena::new(),
            id: 0,

            heading_size: 0,
        }
    }

    pub fn new_id(&mut self) -> usize {
        self.id += 1;

        self.id
    }

    pub fn preorder<V: Visitor>(&self, v: &V) {
        self.tree.preorder_visit(v)
    }

    pub fn parse(&mut self, w: &mut Walker<'_>) {
        let mut pos_before_state_change = 0;
        while let Some(value) = w.peek(0) {
            match value as char {
                '#' => match self.state {
                    State::Paragraph => {
                        let blank = w.position() == 0 || w.peek_back(1) == Some(b'\n');
                        let num = w.till_not(b'#');
                        let is_next_blank = w.is_next_char(b' ');

                        if blank && is_next_blank && num < 7 {
                            w.advance(1); // blank space after `#`s

                            self.heading_size = num as u8;
                            self.state = State::AtxHeading;
                            pos_before_state_change = w.position();
                        }
                    }

                    _ => todo!(),
                },

                any => match self.state {
                    State::Paragraph => {
                        if (any == '\n' && w.peek(1) == Some(b'\n')) || w.peek(1).is_none() {
                            let pos = Position::new(pos_before_state_change, w.position() + 1);
                            let para = AstNode::new(Value::Paragraph, pos, self.new_id());

                            let text = AstNode::new(Value::Text, pos, self.new_id());

                            self.tree.attach_node(para);

                            let _ = self.tree.go_down();

                            self.tree.attach_node(text);

                            self.state = State::Paragraph;

                            pos_before_state_change = w.position();
                            dbg!(w.position());
                            let _ = self.tree.go_up();
                        }

                        w.advance(1);
                    }

                    State::AtxHeading => {
                        if any == '\n' || w.peek(1).is_none() {
                            let pos = Position::new(pos_before_state_change, w.position() + 1);
                            dbg!(pos);
                            let heading = AstNode::new(
                                Value::Heading {
                                    level: NonZero::new(self.heading_size)
                                        .expect("heading_size should be >0"),
                                    atx: true,
                                },
                                pos,
                                self.new_id(),
                            );

                            let text = AstNode::new(Value::Text, pos, self.new_id());

                            self.tree.attach_node(heading);

                            let _ = self.tree.go_down();

                            self.tree.attach_node(text);

                            self.state = State::Paragraph;
                            pos_before_state_change = w.position();
                        }

                        w.advance(1);
                    }

                    _ => todo!(),
                },
            }
        }
    }
}

/// Checks for a newline 1 character back or a lack of any character (start of file)
fn blank_before(w: &mut Walker<'_>) -> bool {
    let peek = w.peek_back(1);

    dbg!(peek.map(|x| x as char));
    peek == Some(b'\n') || peek.is_none()
}

#[cfg(test)]
mod tests {
    use super::{AstNode, Parser, Walker};
    use crate::ast::Value;
    struct Visitor<'a>(&'a str);

    impl crate::tree::Visitor for Visitor<'_> {
        fn visit_node(&self, value: &AstNode) {
            dbg!(value);
            let val = value.as_str(self.0);

            println!("{:#?}: {val}\n", value.value());
        }
    }

    #[test]
    fn paragraph() {
        let data = concat!(
            "First paragraph\n",
            "continuation of the first one\n\n",
            "Second paragraph\n",
            "continuation of the second one"
        );

        dbg!(data.get(0..46));

        let mut w = Walker::new(data);
        let mut p = Parser::new();

        let visitor = Visitor(data);

        p.parse(&mut w);

        p.preorder(&visitor);
    }

    #[test]
    fn heading() {
        let data = concat!("###### Heading level 6\nmeow");
        let mut w = Walker::new(data);
        let mut p = Parser::new();

        let visitor = Visitor(data);

        p.parse(&mut w);

        dbg!(w.position());
        dbg!(p.tree.storage());
        p.preorder(&visitor);
    }
}
