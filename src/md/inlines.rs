pub struct Inlines<'i> {
    list: Vec<Inline<'i>>,
    src: String,
}

impl<'a> Inlines<'a> {
    pub fn new(src: String) -> Self {
        Self {
            list: Vec::new(),
            src,
        }
    }

    pub fn add(&mut self, item: Inline<'a>) {
        self.list.push(item);
    }
}

pub enum Inline<'a> {
    Emph(Emphasis<'a>),
    Link(Link<'a>),
    Image(Image<'a>),
    StrThr(StrikeThrough<'a>),
    Emoji(Emoji<'a>),
    Code(Code<'a>),
    SoftBreak,
    HardBreak,
    Text(Text<'a>),
}

impl<'a> Inline<'a> {
    pub fn emph(strong: bool, delim: char, val: Inline<'a>) -> Inline<'a> {
        Inline::Emph(Emphasis {
            strong,
            delim,
            inner: Box::new(val),
        })
    }

    pub fn link(name: &'a str, target: &'a str) -> Inline<'a> {
        Inline::Link(Link { name, target })
    }

    pub fn image<A>(alt: A, link: &'a str) -> Inline<'a>
    where
        A: Into<Option<&'a str>>,
    {
        Inline::Image(Image {
            alt: alt.into(),
            link,
        })
    }

    pub fn strthr(val: Inline<'a>) -> Inline<'a> {
        Inline::StrThr(StrikeThrough {
            inner: Box::new(val),
        })
    }

    pub fn emoji(name: &'a str) -> Inline<'a> {
        Inline::Emoji(Emoji { name })
    }

    pub fn text(content: &'a str) -> Inline<'a> {
        Inline::Text(Text { content })
    }

    pub fn code(content: &'a str) -> Inline<'a> {
        Inline::Code(Code { content })
    }

    pub fn soft_break() -> Inline<'a> {
        Inline::SoftBreak
    }

    pub fn hard_break() -> Inline<'a> {
        Inline::HardBreak
    }
}

pub struct Emphasis<'a> {
    strong: bool,
    delim: char,
    inner: Box<Inline<'a>>,
}

pub struct Link<'a> {
    name: &'a str,
    target: &'a str,
}

pub struct Image<'a> {
    alt: Option<&'a str>,
    link: &'a str,
}

pub struct StrikeThrough<'a> {
    inner: Box<Inline<'a>>,
}

pub struct Emoji<'a> {
    name: &'a str,
}

pub struct Text<'a> {
    content: &'a str,
}

pub struct Code<'a> {
    content: &'a str,
}
