#[derive(Debug)]
pub enum Phrasing<'p> {
    Break,
    Bold(Box<Phrasing>),
    Italic(Box<Phrasing>),
    Literal(&'p str),
    Code(Code<'p>),
}

#[derive(Debug)]
pub struct Code<'c> {
    lang: &'c str,
    cfg: &'c str,
    val: &'c str,
}
