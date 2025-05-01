#[cfg(test)]
mod tests {
    use crate::{
        block_parser::DefaultParser,
        md::{Block, blocks::code::meta::Lang},
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
        let mut parser = BlockParser::new();

        match parser.block(&mut walker) {
            Block::Blockquote(bq) => {
                match *bq.text.expect("no inner element") {
                    Block::Paragraph(para) => {
                        let text = para.text;
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
            Block::Heading(h) => {
                let text = h.text.expect("no text present in heading");

                assert!(text == "Heading");
                assert!(h.level.map_or(false, |x| u8::from(x.0) == 1));
            }

            any => panic!("block was not a blockquote, was: {:#?}", any),
        };

        match parser.block(&mut walker) {
            Block::Paragraph(para) => {
                assert!(para.text == "#BrokenHeading")
            }

            any => panic!("block was not a paragraph, was: {:#?}", any),
        }

        match parser.block(&mut walker) {
            Block::FencedCode(code) => {
                match code.meta.info {
                    Some(info) => assert!("some_meta_data=noumea :3" == info, "invalid meta data"),
                    _ => panic!("no metadata was found"),
                }

                match code.meta.lang {
                    super::Lang::Rust => {}

                    lang => panic!("invalid language recognised: {lang:#?}"),
                }

                assert!(
                    code.text.is_some_and(|str| str == "panic!()\n"),
                    "wrongly read code block"
                )
            }

            any => panic!("block was not fenced code, was: {:#?}", any),
        };

        match parser.block(&mut walker) {
            Block::IndentedCode(icode) => {
                let text = icode.indents.get(0).expect("only indent was not present");

                assert!(text == "Indented code!");
            }

            any => panic!("block was not `IndentedCode`, was: {:#?}", any),
        }

        match parser.block(&mut walker) {
            Block::StyleBreak(_) => {}

            any => panic!("block was not `StyleBreak`, was: {:#?}", any),
        };

        match parser.block(&mut walker) {
            Block::Heading(hd) => {
                assert!(hd.is_level(1), "wrong heading level");
                assert!(
                    hd.text.is_some_and(|x| x == "Heading with equals"),
                    "invalid heading text"
                );
            }

            any => panic!("block was not `Heading`, was: {:#?}", any),
        }

        match parser.block(&mut walker) {
            Block::Paragraph(para) => assert!(
                para.text == "and let's have a nice paragraph",
                "invalid paragraph text: {0}",
                para.text
            ),

            any => panic!("block was not `Paragraph`, was: {:#?}", any),
        };

        match dbg!(parser.block(&mut walker)) {
            Block::List(ord) => match ord {
                super::List::Ordered(order) => {
                    let items = order.items.into_iter();

                    items.for_each(|item| {
                        match *item.item {
                            Block::Paragraph(parap) => {
                                println!("text:\n{:#?}", parap.text)
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
                super::List::Bullet(b) => {
                    b.items.into_iter().for_each(|x| {
                        let string = match *x.item {
                            Block::Paragraph(p) => p.text,

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

        let mut parser = BlockParser::new();
        let mut walker = Walker::new(md);

        let val = parser.block(&mut walker);

        let inner = match val {
            Block::Blockquote(q) => *q.text.expect("field not present"),
            _ => panic!("block was not blockquote"),
        };

        match inner {
            Block::Paragraph(para) => {
                let text = para.text;

                assert!(text == "This is a blockquote");
            }

            _ => assert!(false, "block was not paragraph"),
        }

        let val = parser.block(&mut walker);

        let inner = match val {
            Block::Blockquote(q) => *q.text.expect("field not present"),
            _ => panic!("block was not blockquote"),
        };

        match inner {
            Block::Paragraph(para) => {
                let text = para.text;

                assert!(text == "This is an another blockquote but a longer one!");
            }

            _ => assert!(false, "block was not paragraph"),
        }
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
        let mut parser = BlockParser::new();

        match dbg!(parser.block(&mut walker)) {
            Block::List(ord) => match ord {
                super::List::Ordered(order) => {
                    let items = order.items.into_iter();

                    items.for_each(|item| {
                        match *item.item {
                            Block::Paragraph(parap) => {
                                println!("text:\n{:#?}", parap.text)
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
        let mut parser = BlockParser::new();

        match parser.block(&mut walker) {
            Block::List(ls) => match ls {
                super::List::Bullet(b) => {
                    b.items.into_iter().for_each(|x| {
                        let string = match *x.item {
                            Block::Paragraph(p) => p.text,

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
        let mut parser = BlockParser::new();

        let block = match parser.block(&mut walker) {
            Block::FencedCode(fc) => fc,

            _ => panic!("block was not fenced code"),
        };

        assert!(block.text.expect("text should be here") == "#[no_std]\n");
    }

    #[test]
    fn code_tilde() {
        let data = concat!("~~~rust\n", "#[no_std]\n", "~~~");

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        let block = match parser.block(&mut walker) {
            Block::FencedCode(fc) => fc,

            _ => panic!("block was not fenced code"),
        };

        assert!(block.text.expect("text should be here") == "#[no_std]\n");
    }

    #[test]
    fn code_indented() {
        let data = concat!(
            "       code line 1\n",
            "       code line 2\n",
            "       code line 3\n",
            "       code line 4\n",
            "       code line 5\n",
        );

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        let block = parser.block(&mut walker);

        let inner = match block {
            Block::IndentedCode(ic) => ic,
            _ => panic!("block was not indented code"),
        };

        inner
            .indents
            .into_iter()
            .enumerate()
            .map(|(index, val)| (index + 1, val))
            .for_each(|(index, value)| {
                let test = format!("code line {}", index);

                assert!(test == value, "wrong value at line: {}", index)
            });
    }

    #[test]
    fn heading_simple() {
        let data = "###### une, grande, et indivisible";

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        let block = match parser.block(&mut walker) {
            Block::Heading(h) => h,

            _ => panic!("block was not a heading"),
        };

        assert!(
            block.is_level(6),
            "invalid level found, was supposed to be 6, is {:#?}",
            block.level
        );

        assert!(
            block.text.expect("should be here") == "une, grande, et indivisible",
            "invalid text in heading"
        );
    }

    #[test]
    fn heading_under() {
        let data = concat!("Heading text\n", "======",);

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

        let block = match parser.block(&mut walker) {
            Block::Heading(h) => h,

            _ => panic!("block was not a heading"),
        };

        assert!(
            block.is_level(1),
            "invalid level found, was supposed to be 1, is {:#?}",
            block.level
        );

        let text = block.text.expect("should be here");
        assert!(
            text == "Heading text",
            "invalid text in heading, was: {text:?}"
        );
    }

    #[test]
    fn style_break_simple() {
        let data = concat!("___\n", "---\n", "***\n");

        let mut walker = Walker::new(data);
        let mut parser = BlockParser::new();

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
        let parser = BlockParser::new();

        let document = parser.document(&mut walker);

        println!("{:#?}", document);
    }
}
