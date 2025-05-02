use std::ops::Index;

use super::walker::StrRange;

#[derive(Debug)]
pub struct Inlines {
    list: Vec<Inline>,
}

impl Inlines {
    pub fn new() -> Self {
        Self { list: Vec::new() }
    }

    pub fn add(&mut self, item: Inline) {
        self.list.push(item);
    }

    pub fn iter_values<'a, 'b>(&'b mut self, data: &'a str) -> impl IntoIterator<Item = &'a str>
    where
        'a: 'b,
    {
        fn grab<'a, 'b>(inl: &'b mut Inline, data: &'a str) -> &'a str
        where
            'a: 'b,
        {
            match inl {
                Inline::Emoji(em) => em.name.resolve(data.as_bytes()),
                Inline::HardBreak => "hard break",
                Inline::SoftBreak => "soft break",
                Inline::Text(txt) => txt.content.resolve(data.as_bytes()),
                Inline::Code(code) => code.content.resolve(data.as_bytes()),

                Inline::Emph(emph) => emph.map_inner(|x| grab(x, data)),
                Inline::StrThr(str) => str.map_inner(|x| grab(x, data)),

                Inline::Link(link) => link.map_str(data, |x| grab(x, data), |_| {}).0,

                Inline::Image(img) => img.link.resolve(data.as_bytes()),
            }
        }

        self.list.iter_mut().map(|val| grab(val, data))
    }
}

impl Default for Inlines {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub enum Inline {
    Emph(Emphasis),
    Link(Link),
    Image(Image),
    StrThr(StrikeThrough),
    Emoji(Emoji),
    Code(Code),
    SoftBreak,
    HardBreak,
    Text(Text),
}

impl Inline {
    pub fn emph(strong: bool, delim: EmphasisChar, val: Inline) -> Inline {
        Inline::Emph(Emphasis {
            strong,
            delim,
            inner: Box::new(val),
        })
    }

    pub fn link(name: Inline, target: StrRange) -> Inline {
        Inline::Link(Link {
            name: Box::new(name),
            target,
        })
    }

    pub fn image<A>(alt: A, link: StrRange) -> Inline
    where
        A: Into<Option<StrRange>>,
    {
        Inline::Image(Image {
            alt: alt.into(),
            link,
        })
    }

    pub fn strthr(val: Inline) -> Inline {
        Inline::StrThr(StrikeThrough {
            inner: Box::new(val),
        })
    }

    pub fn emoji(start: usize, end: usize) -> Inline {
        Inline::Emoji(Emoji {
            name: StrRange::new(start, end),
        })
    }

    pub fn text(start: usize, end: usize) -> Inline {
        Inline::Text(Text {
            content: StrRange::new(start, end),
        })
    }

    pub fn code(start: usize, end: usize) -> Inline {
        Inline::Code(Code {
            content: StrRange::new(start, end),
        })
    }

    pub fn soft_break() -> Inline {
        Inline::SoftBreak
    }

    pub fn hard_break() -> Inline {
        Inline::HardBreak
    }
}

#[derive(Debug)]
pub struct Emphasis {
    strong: bool,
    delim: EmphasisChar,
    inner: Box<Inline>,
}

#[derive(Debug, PartialEq, PartialOrd, Ord, Eq, Copy, Clone)]
pub enum EmphasisChar {
    Asterisk = 0,
    Underscore = 1,
}

impl EmphasisChar {
    pub fn from_u8(val: u8) -> Option<EmphasisChar> {
        match val {
            b'*' => Some(EmphasisChar::Asterisk),
            b'_' => Some(EmphasisChar::Underscore),

            _ => None,
        }
    }

    pub fn to_char(self) -> char {
        unsafe { *['*', '_'].get_unchecked(self as usize) }
    }
}

impl Emphasis {
    pub fn map_inner<F, T>(&mut self, func: F) -> T
    where
        F: FnOnce(&mut Inline) -> T,
    {
        func(self.inner.as_mut())
    }
}

#[derive(Debug)]
pub struct Link {
    name: Box<Inline>,
    target: StrRange,
}

impl Link {
    pub fn map_str<'d, F1, F2, T1, T2>(&mut self, data: &'d str, fl: F1, fr: F2) -> (T1, T2)
    where
        F1: FnOnce(&mut Inline) -> T1 + 'd,
        F2: FnOnce(&str) -> T2 + 'd,
    {
        (
            fl(self.name.as_mut()),
            fr(self.target.resolve(data.as_bytes())),
        )
    }
}

#[derive(Debug)]
pub struct Image {
    alt: Option<StrRange>,
    link: StrRange,
}

impl Image {
    pub fn map_str<'d, F1, F2, T1, T2>(&self, data: &'d str, fl: F1, fr: F2) -> (Option<T1>, T2)
    where
        F1: FnOnce(&str) -> T1 + 'd,
        F2: FnOnce(&str) -> T2 + 'd,
    {
        let left = if let Some(ref val) = self.alt {
            fl(val.resolve(data.as_bytes())).into()
        } else {
            None
        };

        let right = fr(self.link.resolve(data.as_bytes()));

        (left, right)
    }
}

#[derive(Debug)]
pub struct StrikeThrough {
    inner: Box<Inline>,
}

impl StrikeThrough {
    pub fn map_inner<F, T>(&mut self, func: F) -> T
    where
        F: FnOnce(&mut Inline) -> T,
    {
        func(self.inner.as_mut())
    }
}

#[derive(Debug)]
pub struct Emoji {
    name: StrRange,
}

impl Emoji {
    pub fn map_str<F, T>(&self, data: &str, func: F) -> T
    where
        F: FnOnce(&str) -> T,
    {
        func(self.name.resolve(data.as_bytes()))
    }
}

#[derive(Debug)]
pub struct Text {
    content: StrRange,
}

impl Text {
    pub fn map_str<F, T>(&self, data: &str, func: F) -> T
    where
        F: FnOnce(&str) -> T,
    {
        func(self.content.resolve(data.as_bytes()))
    }
}

#[derive(Debug)]
pub struct Code {
    content: StrRange,
}

impl Code {
    pub fn map_str<F, T>(&self, data: &str, func: F) -> T
    where
        F: FnOnce(&str) -> T,
    {
        func(self.content.resolve(data.as_bytes()))
    }
}
