#[cfg(test)]
mod tests {
    use crate::{
        block_parser::{BlockParser, DefaultParser},
        md::{Block, blocks::code::meta::Lang, blocks::lists::List},
        walker::Walker,
    };

    #[test]
    fn complete() {
        let data = concat!(
            "> Blockquote\n",
            ">BlockquoteNoSpace\n",
            "# Heading\n",
            "#BrokenHeading\n",
            "```rust,some_meta_data=noumea :3\n",
            "panic!()\n",
            "```\n",
            "    Indented code!\n",
            "--*\n",
            "Heading with equals\n",
            "======\n",
            "and let's have a nice paragraph\n",
            "1) Order 1\n",
            "2) Order 2\n",
            "3) Order 3\n",
            "4) Order 4\n",
            "+ Meow\n",
            "+ Awrff\n",
            "+ Bark\n"
        );

        let mut walker = Walker::new(data);
        let mut parser = DefaultParser::new();

        match parser.block(&mut walker) {
            Block::Blockquote(mut bq) => {
                match bq.inner().expect("no inner element") {
                    Block::Paragraph(para) => {
                        let text = para.inner();
                        assert!(
                            "Blockquote BlockquoteNoSpace" == text,
                            "invalid text, was: {text}"
                        );
                    }

                    _ => panic!("inner block was not a paragraph"),
                };
            }

            any => panic!("block was not a blockquote, was: {:#?}", any),
        };

        match parser.block(&mut walker) {
            Block::Heading(mut h) => {
                let text = h.inner().expect("no text present in heading");

                assert!(text == "Heading");
                assert!(h.level().map_or(false, |x| u8::from(x) == 1));
            }

            any => panic!("block was not a blockquote, was: {:#?}", any),
        };

        match parser.block(&mut walker) {
            Block::Paragraph(mut para) => {
                assert!(para.inner() == "#BrokenHeading")
            }

            any => panic!("block was not a paragraph, was: {:#?}", any),
        }

        match parser.block(&mut walker) {
            Block::FencedCode(mut code) => {
                match code.meta().info() {
                    Some(info) => assert!("some_meta_data=noumea :3" == info, "invalid meta data"),
                    _ => panic!("no metadata was found"),
                }

                match code.meta().lang() {
                    Lang::Rust => {}

                    lang => panic!("invalid language recognised: {lang:#?}"),
                }

                assert!(
                    code.inner().is_some_and(|str| str == "panic!()\n"),
                    "wrongly read code block"
                )
            }

            any => panic!("block was not fenced code, was: {:#?}", any),
        };

        match parser.block(&mut walker) {
            Block::IndentedCode(icode) => {
                let text = icode.indents().get(0).expect("only indent was not present");

                assert!(text == "Indented code!");
            }

            any => panic!("block was not `IndentedCode`, was: {:#?}", any),
        }

        match parser.block(&mut walker) {
            Block::StyleBreak(_) => {}

            any => panic!("block was not `StyleBreak`, was: {:#?}", any),
        };

        match parser.block(&mut walker) {
            Block::Heading(mut hd) => {
                assert!(hd.is_level(1), "wrong heading level");
                assert!(
                    hd.inner().is_some_and(|x| x == "Heading with equals"),
                    "invalid heading text"
                );
            }

            any => panic!("block was not `Heading`, was: {:#?}", any),
        }

        match parser.block(&mut walker) {
            Block::Paragraph(mut para) => assert!(
                para.inner() == "and let's have a nice paragraph",
                "invalid paragraph text: {0}",
                para.inner()
            ),

            any => panic!("block was not `Paragraph`, was: {:#?}", any),
        };

        match dbg!(parser.block(&mut walker)) {
            Block::List(ord) => match ord {
                List::Ordered(mut order) => {
                    let items = order.items_mut().into_iter();

                    items.for_each(|item| {
                        match item.inner() {
                            Block::Paragraph(parap) => {
                                println!("text:\n{:#?}", parap.inner())
                            }

                            _ => panic!("was not paragraph"),
                        };
                    });
                }

                _ => panic!("list was not ordered"),
            },

            _ => panic!("block was not an ordered list"),
        };

        match parser.block(&mut walker) {
            Block::List(ls) => match ls {
                List::Bullet(mut b) => {
                    b.items_mut().into_iter().for_each(|x| {
                        let string = match x.inner() {
                            Block::Paragraph(p) => p.inner(),

                            any => panic!("not paragraph, is: {any:#?}"),
                        };

                        dbg!(string);
                    });
                }

                any => panic!("not bullet list, is: {any:#?} "),
            },

            any => panic!("not list, is: {any:#?}"),
        };

        // match parser.block(&mut walker) {
        //     Block::Eof => {}

        //     any => panic!("not EOF, block returned was: {:#?}", any),
        // };
    }

    #[test]
    fn blockquote() {
        let md = concat!(
            ">>> This is a blockquote\n",
            ">>>> This is an another blockquote\nbut a longer one!",
        );

        let mut parser = DefaultParser::new();
        let mut walker = Walker::new(md);

        match parser.block(&mut walker) {
            Block::Blockquote(mut q) => {
                let mut para = q.inner().expect("field not present");

                match &mut para {
                    Block::Paragraph(para) => {
                        let text = para.inner();

                        assert!(text == "This is a blockquote");
                    }

                    _ => assert!(false, "block was not paragraph"),
                }
            }
            _ => panic!("block was not blockquote"),
        };

        match parser.block(&mut walker) {
            Block::Blockquote(mut q) => {
                let mut para = q.inner().expect("field not present");

                match &mut para {
                    Block::Paragraph(para) => {
                        let text = para.inner();

                        assert!(text == "This is an another blockquote but a longer one!");
                    }

                    _ => assert!(false, "block was not paragraph"),
                }
            }
            _ => panic!("block was not blockquote"),
        };
    }

    #[test]
    fn ordered_list() {
        let data = concat!(
            "1) Niente dei, niente padroni\n",
            "2) No gods, no masters\n",
            "3) Ni dieu, ni maitre\n",
            "4) Ani boga, ani pana\n",
        );

        let mut walker = Walker::new(data);
        let mut parser = DefaultParser::new();

        match dbg!(parser.block(&mut walker)) {
            Block::List(ord) => match ord {
                List::Ordered(mut order) => {
                    let items = order.items_mut().into_iter();

                    items.for_each(|item| {
                        match item.inner() {
                            Block::Paragraph(parap) => {
                                println!("text:\n{:#?}", parap.inner())
                            }

                            _ => panic!("was not paragraph"),
                        };
                    });
                }

                _ => panic!("list was not ordered"),
            },

            _ => panic!("block was not an ordered list"),
        };
    }

    #[test]
    fn bullet_list() {
        let data = concat!("+ Meow\n", "+ Awrff\n", "+ Bark\n");

        let mut walker = Walker::new(data);
        let mut parser = DefaultParser::new();

        match parser.block(&mut walker) {
            Block::List(ls) => match ls {
                List::Bullet(mut b) => {
                    b.items_mut().into_iter().for_each(|x| {
                        let string = match x.inner() {
                            Block::Paragraph(p) => p.inner(),

                            _ => panic!("not paragraph"),
                        };

                        dbg!(string);
                    });
                }

                _ => panic!("not bullet list"),
            },

            _ => panic!("not list"),
        };
    }

    #[test]
    fn code() {
        let data = concat!("```rust\n", "#[no_std]\n", "```");

        let mut walker = Walker::new(data);
        let mut parser = DefaultParser::new();

        let mut block = match parser.block(&mut walker) {
            Block::FencedCode(fc) => fc,

            _ => panic!("block was not fenced code"),
        };

        assert!(block.inner().expect("text should be here") == "#[no_std]\n");
    }

    #[test]
    fn code_tilde() {
        let data = concat!("~~~rust\n", "#[no_std]\n", "~~~");

        let mut walker = Walker::new(data);
        let mut parser = DefaultParser::new();

        let mut block = match parser.block(&mut walker) {
            Block::FencedCode(fc) => fc,

            _ => panic!("block was not fenced code"),
        };

        assert!(block.inner().expect("text should be here") == "#[no_std]\n");
    }

    #[test]
    fn code_indented() {
        let data = concat!(
            "       code line 1\n",
            "       code line 2\n",
            "       code line 3\n",
            "       code line 4\n",
            "       code line 5\n",
            "    code line 6\n", // these two are with tabs
            "    code line 7\n",
        );

        let mut walker = Walker::new(data);
        let mut parser = DefaultParser::new();

        let block = parser.block(&mut walker);

        let inner = match block {
            Block::IndentedCode(ic) => dbg!(ic),
            _ => panic!("block was not indented code"),
        };

        inner
            .indents()
            .into_iter()
            .enumerate()
            .map(|(index, val)| (index + 1, val))
            .for_each(|(index, value)| {
                let test = format!("code line {}", index);

                assert!(&test == value, "wrong value at line: {}", index)
            });
    }

    #[test]
    fn heading_simple() {
        let data = "###### une, grande, et indivisible";

        let mut walker = Walker::new(data);
        let mut parser = DefaultParser::new();

        let mut block = match parser.block(&mut walker) {
            Block::Heading(h) => h,

            _ => panic!("block was not a heading"),
        };

        assert!(
            block.is_level(6),
            "invalid level found, was supposed to be 6, is {:#?}",
            block.level()
        );

        assert!(
            block.inner().expect("should be here") == "une, grande, et indivisible",
            "invalid text in heading"
        );
    }

    #[test]
    fn heading_under() {
        let data = concat!("Heading text\n", "======",);

        let mut walker = Walker::new(data);
        let mut parser = DefaultParser::new();

        let mut block = match parser.block(&mut walker) {
            Block::Heading(h) => h,

            _ => panic!("block was not a heading"),
        };

        assert!(
            block.is_level(1),
            "invalid level found, was supposed to be 1, is {:#?}",
            block.level()
        );

        let text = block.inner().expect("should be here");
        assert!(
            text == "Heading text",
            "invalid text in heading, was: {text:?}"
        );
    }

    #[test]
    fn style_break_simple() {
        let data = concat!("___\n", "---\n", "***\n");

        let mut walker = Walker::new(data);
        let mut parser = DefaultParser::new();

        match parser.block(&mut walker) {
            Block::StyleBreak(_) => {}

            _ => panic!("block was not style break"),
        };

        match parser.block(&mut walker) {
            Block::StyleBreak(_) => {}

            _ => panic!("block was not style break"),
        };

        match parser.block(&mut walker) {
            Block::StyleBreak(_) => {}

            _ => panic!("block was not style break"),
        };
    }

    #[test]
    fn html_blocks() {
        let data = concat!(
            "<pre va>this is some serious content</pre>\n",
            "<script \t this is some serious content 2</script>\n",
            "<textarea \t this is some serious content</textarea>\n",
            "<style \t this is some serious content</style>\n",
            "<!-- html comment -->\n",
            "<? whatever ?>\n",
            "<!block>\n",
            "<![CDATA[ \"L'Alsace et la Lorraine\" ]]>\n",
            "</address \t>\n\n",
            "<vabank\n>\n\n",
            "</hr>\n\n",
            "<br>\n\n",
            "Paragraph test\n<!test>"
        );

        let mut walker = Walker::new(data);
        let parser = DefaultParser::new();

        let document = parser.document(&mut walker);

        println!("{:#?}", document);
    }

    #[test]
    fn nested() {
        let data = concat!(
            "> 1. First list entry\n",
            "> 2. Second list entry\n",
            "3. Third list entry\n",
            "> + Bullet list 1\n",
            "> + Bullet list 2\n",
            "> + Bullet list 3\n",
            "+ Outside blist 1\n\n",
            "+ Not tight!\n",
            "+ > # meow",
        );

        let p = DefaultParser::new();
        let mut w = Walker::new(data);

        let doc = p.document(&mut w);

        println!("{:#?}", doc);
    }
}
