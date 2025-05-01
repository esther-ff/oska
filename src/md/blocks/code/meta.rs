#[derive(Debug)]
pub struct CodeMeta {
    lang: Lang,
    info: Option<String>,
}

impl CodeMeta {
    pub fn new<A: Into<Option<String>>>(lang: Lang, info: A) -> Self {
        Self {
            info: info.into(),
            lang,
        }
    }
}

#[derive(Debug)]
pub enum Lang {
    None,
    Rust,
    NotSupported(Box<str>),
}

impl Lang {
    pub fn is_useless(&self) -> bool {
        matches!(self, Self::None | Self::NotSupported(_))
    }

    pub fn recognize(name: &str) -> Lang {
        match name {
            "rust" => Lang::Rust,

            "" => Lang::None,

            unknown => Lang::NotSupported(unknown.to_string().into_boxed_str()),
        }
    }
}
