pub mod blockquote;
pub mod code;
pub mod heading;
pub mod html_block;
pub mod lists;
pub mod paragraph;
pub mod style_break;

pub(crate) mod utils;

use core::marker::PhantomData;

#[derive(Debug)]
pub struct Parsed;
#[derive(Debug)]
pub struct Unparsed;

use blockquote::{BlkQt, BlkQtLevel};
use code::{fenced::Code, indented::IndentCode};
use heading::{Heading, HeadingLevel};
use html_block::*;
use lists::{bullet_list::BulletList, ordered_list::OrderedList};
use paragraph::Paragraph;
use style_break::Break;

/// A enum representing all implemented types of Markdown
/// blocks.
#[derive(Debug)]
pub enum Block<State> {
    Paragraph(paragraph::Paragraph),
    Blockquote(blockquote::BlkQt),
    List(lists::List),
    FencedCode(code::fenced::Code),
    IndentedCode(code::indented::IndentCode),
    Heading(heading::Heading),
    StyleBreak(style_break::Break),
    HtmlBlock(html_block::HtmlBlock),
    Eof,

    _State(PhantomData<State>),
}

impl Block<Unparsed> {
    #[inline]
    pub fn make_paragraph(text: String, id: usize) -> Block<Unparsed> {
        Block::Paragraph(Paragraph { text, id })
    }

    #[inline]
    pub fn make_blockquote(
        range: impl Into<Option<Block<Unparsed>>>,
        id: usize,
        level: usize,
    ) -> Block<Unparsed> {
        Block::Blockquote(BlkQt {
            level: BlkQtLevel::new(level),
            text: range.into().map(Box::new),
            id,
        })
    }

    #[inline]
    pub fn make_ordered_list(
        start_number: usize,
        items: Vec<ListItem>,
        tight: bool,
        id: usize,
    ) -> Block<Unparsed> {
        Block::List(List::Ordered(OrderedList {
            tight,
            start_number,
            items,
            id,
        }))
    }

    #[inline]
    pub fn make_bullet_list(items: Vec<ListItem>, tight: bool, id: usize) -> Block<Unparsed> {
        Block::List(List::Bullet(BulletList { tight, items, id }))
    }

    #[inline]
    pub fn make_code(
        code: impl Into<Option<String>>,
        meta: impl Into<Option<String>>,
        lang: Lang,
        id: usize,
    ) -> Block<Unparsed> {
        let meta = CodeMeta {
            lang,
            info: meta.into(),
        };

        Block::FencedCode(Code {
            meta,
            text: code.into(),
            id,
        })
    }

    #[inline]
    pub fn make_indented_code<T: Into<Box<[String]>>>(indents: T, id: usize) -> Block<Unparsed> {
        Block::IndentedCode(IndentCode {
            indents: indents.into(),
            id,
        })
    }

    #[inline]
    pub fn make_heading(
        range: impl Into<Option<String>>,
        heading_level: impl Into<Option<u8>>,
        id: usize,
    ) -> Block<Unparsed> {
        Block::Heading(Heading {
            level: heading_level.into().map(HeadingLevel::new),
            text: range.into(),
            id,
        })
    }

    #[inline]
    pub fn make_style_break(id: usize) -> Block<Unparsed> {
        Block::StyleBreak(Break { id })
    }

    #[inline]
    pub fn make_html_block(inner: String, id: usize) -> Block<Unparsed> {
        Block::HtmlBlock(HtmlBlock { inner, id })
    }
}
