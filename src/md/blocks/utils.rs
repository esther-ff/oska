use crate::md::chars::{BACKTICK, GREATHER_THAN, HASH, LESSER_THAN, NEWLINE, SPACE};
use crate::md::walker::Walker;

/// Checks for a the possibility of a new block
/// attempts to be "pure"
/// if it returns false, the state of the parser
/// is the same as it was before the call
/// if it is true, the state may or may not be different
fn check_for_possible_new_block(walker: &mut Walker<'_>) -> bool {
    let next = match walker.peek(0) {
        None => return false,
        Some(val) => val,
    };

    match next {
        NEWLINE => {
            walker.advance(1);
            true
        }

        BACKTICK => {
            // let pos = walker.position();
            let amnt_of_backticks = walker.till_not(BACKTICK);

            if amnt_of_backticks <= 3 {
                walker.retreat(amnt_of_backticks);

                true
            } else {
                false
            }
        }

        HASH => {
            let amnt_of_hashes = walker.till_not(HASH);
            let is_after_space = walker.is_next_char(SPACE);

            if 6 > amnt_of_hashes && is_after_space {
                walker.retreat(amnt_of_hashes);
                true
            } else {
                false
            }
        }

        char if char.is_ascii_digit() => {
            walker.advance(1);
            let val = is_ordered_list_indicator(walker);
            walker.retreat(1);

            val
        }

        char if is_bullet_list_marker(char) => walker.peek(1) == Some(SPACE),

        LESSER_THAN => {
            let pos = walker.position();
            let jump = walker.is_next_char(b'/') as usize;
            walker.advance(jump);

            if walker
                .peek(0)
                // this generally is a pretty bad workaround around the case-insensitiveness
                // and it isn't even utilised yet
                // (look at the iterated array)
                // Some way to do it would be nice
                // maybe in the walker
                .is_some_and(|target| matches!(target, b's' | b't' | b'p' | b'S' | b'T' | b'P'))
            {
                for pat in ["pre", "script", "style", "textarea"] {
                    if walker.find_string(pat) {
                        walker.set_position(pos);
                        return false;
                    }
                }
            }

            walker.set_position(pos);

            true
        }

        _ => false,
    }
}

/// Used after a numeric character
/// returns true if the 2 next characters are either `. ` or `) `.
/// does not advance the position of the walker.
fn is_ordered_list_indicator(walker: &mut Walker<'_>) -> bool {
    walker.is_next_pred(|val| matches!(val, DOT | RIGHT_PAREN))
        && walker.peek(1).is_some_and(|char| char == SPACE)
}

/// Checks if the given character
/// can be a bullet list marker
/// (means: `+` or `-` or `*`)
fn is_bullet_list_marker(victim: u8) -> bool {
    matches!(victim, ASTERISK | LINE | PLUS)
}

/// Checks if a blank line is present
/// currently only handles blank lines made by using 2 `\n`s
///
/// TODO: make it handle only space or tab lines
fn is_blank_line(walker: &mut Walker<'_>) -> bool {
    let pred = |x| x == NEWLINE;
    let val = walker.peek(0).is_some_and(pred) && walker.peek(1).is_some_and(pred);

    walker.advance(val as usize + val as usize);

    val
}
