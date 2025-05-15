pub mod blockquote;
pub mod code;
pub mod heading;
pub mod html_block;
pub mod lists;
pub mod paragraph;
pub mod style_break;

use crate::lib::{String, Vec};

pub(crate) mod utils;

use core::marker::PhantomData;

#[derive(Debug)]
pub struct Parsed;
#[derive(Debug)]
pub struct Unparsed;

use blockquote::{BlkQt, BlkQtLevel};
use code::{
    fenced::Code,
    indented::IndentCode,
    meta::{CodeMeta, Lang},
};
use heading::{Heading, HeadingLevel};
use html_block::*;
use lists::{
    bullet_list::{self, BulletList},
    ordered_list::OrderedList,
};
use paragraph::Paragraph;
use style_break::Break;

/// A enum representing all implemented types of Markdown
/// blocks.
#[derive(Debug)]
pub enum Block<State> {
    Paragraph(paragraph::Paragraph),
    Blockquote(blockquote::BlkQt<State>),
    List(lists::List<State>),
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
        Block::Paragraph(paragraph::make_paragraph(text, id))
    }

    #[inline]
    pub fn make_blockquote(
        range: impl Into<Option<Block<Unparsed>>>,
        id: usize,
        level: usize,
    ) -> Block<Unparsed> {
        Block::Blockquote(blockquote::make_blockquote(range, level, id))
    }

    #[inline]
    pub fn make_ordered_list(
        start_number: usize,
        items: Vec<lists::list_item::ListItem<Unparsed>>,
        tight: bool,
        id: usize,
    ) -> Block<Unparsed> {
        Block::List(lists::List::Ordered(OrderedList::new(
            tight,
            start_number,
            items,
            id,
        )))
    }

    #[inline]
    pub fn make_bullet_list(
        items: Vec<lists::list_item::ListItem<Unparsed>>,
        tight: bool,
        id: usize,
    ) -> Block<Unparsed> {
        Block::List(lists::List::Bullet(bullet_list::make_bullet_list(
            items, tight, id,
        )))
    }

    #[inline]
    pub fn make_code(
        code: impl Into<Option<String>>,
        metadata: impl Into<Option<String>>,
        lang: Lang,
        id: usize,
    ) -> Block<Unparsed> {
        let meta = CodeMeta::new(lang, metadata);
        Block::FencedCode(Code::new(meta, code, id))
    }

    #[inline]
    pub fn make_indented_code(indents: Vec<String>, id: usize) -> Block<Unparsed> {
        Block::IndentedCode(IndentCode::new(indents, id))
    }

    #[inline]
    pub fn make_heading(
        range: impl Into<Option<String>>,
        heading_level: impl Into<Option<u8>>,
        id: usize,
    ) -> Block<Unparsed> {
        Block::Heading(Heading::new(
            heading_level.into().map(HeadingLevel::new),
            range,
            id,
        ))
    }

    #[inline]
    pub fn make_style_break(id: usize) -> Block<Unparsed> {
        Block::StyleBreak(Break::new(id))
    }

    #[inline]
    pub fn make_html_block(inner: String, id: usize) -> Block<Unparsed> {
        Block::HtmlBlock(HtmlBlock::new(inner, id))
    }
}
