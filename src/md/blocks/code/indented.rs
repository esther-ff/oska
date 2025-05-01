use crate::md::BlockParser;
use crate::md::blocks::{Block, Parsed, Unparsed};
use crate::md::chars::{NEWLINE, SPACE};
use crate::md::walker::Walker;

#[derive(Debug)]
pub struct IndentCode {
    indents: Box<[String]>,
    id: usize,
}

impl IndentCode {
    pub fn new(lines: Vec<String>, id: usize) -> Self {
        Self {
            indents: lines.into_boxed_slice(),
            id,
        }
    }

    pub fn indents_vec<F>(&mut self, func: F)
    where
        F: FnOnce(&mut Vec<String>),
    {
        let ptr = self.indents.as_mut_ptr();
        let (length, capacity) = (self.indents.len(), self.indents.len());

        unsafe {
            let mut vec = Vec::from_raw_parts(ptr, length, capacity);

            func(&mut vec)
        }
    }

    pub fn indents_mut(&mut self) -> &mut [String] {
        &mut self.indents
    }

    pub fn indents(&self) -> &[String] {
        &self.indents
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

pub fn indented_code(
    parser: &mut impl BlockParser,
    walker: &mut Walker<'_>,
) -> Option<Block<Unparsed>> {
    let amnt_of_spaces = walker.till_not(SPACE);

    if amnt_of_spaces < 3 {
        return None;
    }

    let mut lines = Vec::with_capacity(16);

    let entry = String::from(walker.till_inclusive(NEWLINE));
    lines.push(entry);

    walker.advance(1);

    loop {
        let amnt_of_spaces = walker.till_not(SPACE);

        if amnt_of_spaces < 4 {
            walker.retreat(amnt_of_spaces);
            break;
        }

        let entry = walker.till_inclusive(NEWLINE);
        lines.push(String::from(entry));
        walker.advance(1);
    }

    Block::make_indented_code(lines, parser.get_new_id()).into()
}
