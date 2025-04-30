use crate::md::chars::{NEWLINE, SPACE};

#[derive(Debug)]
pub struct IndentCode {
    indents: Box<[String]>,
    id: usize,
}

pub fn indented_code(&mut self, walker: &mut Walker<'_>) -> Option<Block<Unparsed>> {
    let amnt_of_spaces = walker.till_not(SPACE);

    if amnt_of_spaces < 3 {
        walker.retreat(amnt_of_spaces);
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

    Block::make_indented_code(lines.into_boxed_slice(), self.get_new_id()).into()
}
