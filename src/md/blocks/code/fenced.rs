use super::meta::CodeMeta;
use crate::md::chars::{BACKTICK, TILDE};
use crate::md::walker::Walker;

#[derive(Debug)]
pub struct Code {
    meta: CodeMeta,
    text: Option<String>,
    id: usize,
}

pub fn fenced_code<const CHAR: u8>(&mut self, walker: &mut Walker<'_>) -> Block<Unparsed> {
    debug_assert!(
        CHAR == TILDE || CHAR == BACKTICK,
        "invalid char provided to the `code` function"
    );

    let amnt_of_backticks = walker.till_not(CHAR);

    if amnt_of_backticks < 2 {
        walker.retreat(amnt_of_backticks + 1);
        return self.paragraph(walker);
    }

    let pos = walker.position();
    let mut lang = Lang::None;
    let mut info = None;

    while let Some(char) = walker.next() {
        if char == CHAR {
            walker.set_position(pos);
            return self.paragraph(walker);
        }

        if char == NEWLINE {
            let range = StrRange::new(pos, walker.position() - 1);

            let mut split = range.resolve(walker.data()).split(',');

            lang = Lang::recognize(
                split
                    .next()
                    .expect("the first part of a `Split` iterator should be here"),
            );
            info = split.next().map(ToOwned::to_owned);

            break;
        }
    }

    let code_start = walker.position();
    let mut code_end = walker.position();

    while let Some(_char) = walker.next() {
        if walker.is_next_char(CHAR) {
            let amnt_of = walker.till_not(CHAR);

            if amnt_of >= amnt_of_backticks {
                code_end = walker.position() - amnt_of;
                walker.advance(1);
                break;
            }
        }
    }

    let string = String::from(walker.get(code_start, code_end));

    Block::make_code(string, info, lang, self.get_new_id())
}
