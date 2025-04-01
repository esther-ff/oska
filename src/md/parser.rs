enum Content<'a> {
    Bold(Box<Bold<'a>>),
    Italic(Box<Italic<'a>>),
    InlineCode(InlineCode<'a>),
    Paragraph(Paragraph<'a>),
}

enum Flow<'f> {
    BlockQuote(BlockQuote<'f>),
    Code(Code<'f>),
}

pub(crate) struct Heading<'a> {
    level: u8,
    inner: Content<'a>,
}

pub(crate) struct Bold<'a> {
    inner: Content<'a>,
}

pub(crate) struct Italic<'a> {
    inner: Content<'a>,
}

pub(crate) struct BlockQuote<'a> {
    inner: Content<'a>,
}

pub(crate) struct InlineCode<'a> {
    val: &'a str,
}

pub(crate) struct Code<'a> {
    lang: &'a str,
    val: &'a str,
}

pub(crate) struct Paragraph<'a> {
    val: &'a str,
}
