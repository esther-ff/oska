use std::num::{NonZero, NonZeroU8};

pub(crate) struct Input<'i> {
    bytes: &'i [u8],
    pub consumed: usize,
}

impl<'i> Input<'i> {
    pub(crate) fn new<A>(data: &'i A) -> Self
    where
        A: AsRef<[u8]> + 'i + ?Sized,
    {
        Self {
            bytes: data.as_ref(),
            consumed: 0,
        }
    }

    // gives a reference to the byte slice offset by `consumed`
    pub(crate) fn leftover(&self) -> &'i [u8] {
        &self.bytes[self.consumed..]
    }

    // checks if we're at the end of the input
    pub(crate) fn eof(&self) -> bool {
        self.consumed >= self.bytes.len()
    }

    // checks if we have a style break
    //
    // if successful, returns the index after the break's newline
    pub(crate) fn scan_style_break(&self) -> Option<usize> {
        let mut ix = 0;
        let mut ret = None;

        if self.eof() {
            return None;
        };

        for byte in self.leftover() {
            match *byte {
                b'+' | b'-' | b'*' => ix += 1,
                b'\n' if ix >= 3 => ret = Some(ix),
                b'\n' => break,

                _ => break,
            }
        }

        if self.leftover().len() == ix {
            ret = Some(ix)
        }

        ret
    }

    // scans for an empty line ending with '\n'
    //
    // TODO: make this work for `\r` too
    pub(crate) fn scan_empty_line(&self) -> Option<usize> {
        let mut ix = 0;
        for byte in self.leftover() {
            match *byte as char {
                '\n' => return Some(ix + 1),
                ' ' | '\t' => ix += 1,
                _ => return None,
            }
        }

        None
    }

    // scans for a line of `=` or `-`
    //
    // if successful, returns (level of heading, index of it's end)
    pub(crate) fn scan_setext_heading(&self) -> Option<(NonZeroU8, usize)> {
        if self.eof() {
            return None;
        }

        let bytes = self.leftover();
        let target_char = if let Some(target) = bytes.first().copied()
            && matches!(target, b'=' | b'-')
        {
            target
        } else {
            return None;
        };

        let level = match target_char {
            b'=' => NonZero::new(1).unwrap(),
            b'-' => NonZero::new(2).unwrap(),
            _ => return None,
        };

        let mut ix = 1;
        for byte in &bytes[1..] {
            match *byte {
                ch if ch == target_char => ix += 1,
                b'\n' => {
                    ix += 1;
                    break;
                }

                _ => return None,
            }
        }

        Some((level, ix))
    }

    // scans for a macro invocation
    //
    // the `MacroSpan` contains the positions of
    //
    // <>= name (arguments) (
    // ^^^ ^^^  ^---------^ x
    // and the returned usize is the index after the brace (marked as x)
    pub(crate) fn scan_macro(&self) -> Option<(MacroSpan, usize)> {
        if self.eof() {
            return None;
        }

        let bytes = self.leftover();
        if bytes.get(0..4).is_none_or(|arr| arr != b"<>= ") {
            return None;
        }

        let mut ix = 4;
        let mut span = MacroSpan {
            operator: (0, 4),
            name: (4, 0),
            args: (0, 0),
        };

        // name
        for byte in &bytes[ix..] {
            if *byte == b' ' {
                span.name.1 = ix + 1;
                ix += 2;
                span.args.0 = ix;
                break;
            }

            ix += 1
        }

        if ix == 3 || bytes.get(ix - 1).is_none_or(|byte| *byte != b'(') {
            return None;
        }

        // arg braces
        for byte in &bytes[ix + 1..] {
            ix += 1;
            if *byte == b')' {
                span.args.1 = ix;
                break;
            }
        }

        // skip whitespace
        loop {
            if bytes.get(ix).is_none_or(|x| !x.is_ascii_whitespace()) {
                ix += 2;
                break;
            }

            ix += 1;
        }

        if bytes.get(ix).copied() != Some(b'(') {
            None
        } else {
            Some((span, ix + 1))
        }
    }

    pub(crate) fn scan_macro_end(&self) -> bool {
        if let Some(arr) = self.bytes.get(self.consumed..) {
            return arr.first().is_some_and(|x| *x == b')')
                && arr.get(1).is_none_or(|x| *x == b'\n');
        }

        false
    }

    // scans for two consecutive newlines like `\n\n`
    pub(crate) fn scan_two_newlines(&self) -> bool {
        if self.eof() {
            return false;
        }

        let bytes = self.leftover();

        bytes
            .first()
            .copied()
            .zip(bytes.get(1).copied())
            .is_some_and(|tuple| tuple == (b'\n', b'\n'))
    }

    // scans for a bullet list start `<char> ` where char is `+` or `-` or `*`
    //
    // if it succeeds, it returns (index after marker, character used, tightness)
    pub(crate) fn scan_bullet_list(&self) -> Option<(usize, char, bool)> {
        let tight = self.scan_two_newlines();
        let mut relative_index = 2;
        if self.eof() {
            return None;
        }

        let mut bytes = self.leftover();

        bytes = if tight {
            relative_index = 4;

            &bytes[2..]
        } else {
            bytes
        };

        let list_marker_byte = bytes.first().copied().map(|byte| byte as char);

        if list_marker_byte.is_some_and(|x| matches!(x, '-' | '+' | '*'))
            && bytes
                .get(1)
                .copied()
                .is_some_and(|byte| matches!(byte as char, ' '))
        {
            Some((relative_index, list_marker_byte?, tight))
        } else {
            None
        }
    }

    // scans for a ordered list start `<num><char> ` where char is `(` or `.`.
    //
    // if it succeeds, it returns (index after marker, character used, index, tightness)
    pub(crate) fn scan_ordered_list(&self) -> Option<(usize, char, u64, bool)> {
        let tight = self.scan_two_newlines();
        let mut ix = if tight { 2 } else { 0 };

        let bytes = &self.bytes.get(self.consumed..)?.get(ix..)?;

        for (i, byte) in bytes.iter().enumerate() {
            if !byte.is_ascii_digit() {
                if *byte != b'.' && *byte != b')' {
                    return None;
                }

                ix += 1;

                break;
            }

            ix = i;
        }

        let marker_char: char = bytes.get(ix).copied().map(Into::into)?;

        if ix == 0
            || ix >= 9
            || bytes
                .get(ix + 1)
                .copied()
                .is_none_or(|bytechar| bytechar != b' ')
        {
            return None;
        }

        let start_num = unsafe {
            str::from_utf8_unchecked(&bytes[..ix])
                .parse::<u64>()
                .expect("infallible")
        };

        Some((ix + 2, marker_char, start_num, tight))
    }

    // scans for a blockquote marker `> ` or `> `
    //
    // returns index after marker if it succeeds
    pub(crate) fn scan_blockquote(&self) -> Option<usize> {
        if self.eof() {
            return None;
        }

        let bytes = self.leftover();
        if bytes
            .first()
            .copied()
            .is_some_and(|bytechar| bytechar == b'>')
        {
            Some(
                1 + usize::from(
                    bytes
                        .get(1)
                        .copied()
                        .is_some_and(|bytechar| bytechar == b' '),
                ),
            )
        } else {
            None
        }
    }

    // scans for a start of an atx heading
    //
    // if it succeeds, returns the index after the marker and all the whitespace
    pub(crate) fn scan_atx_heading(&self) -> Option<usize> {
        if self.eof() {
            return None;
        }

        let bytes = self.leftover();

        let mut ix = 0;

        while bytes.get(ix).copied().is_some_and(|byte| byte == b'#') {
            ix += 1;
        }

        if ix == 0
            || ix > 6
            || bytes.get(ix).is_none()
            || bytes.get(ix).copied().is_some_and(|x| x != b' ')
        {
            return None;
        }

        // consume all the white space
        while bytes.get(ix).copied().is_some_and(|byte| byte == b' ') {
            ix += 1;
        }

        Some(ix)
    }

    // scans for a condition that would mean the interruption of a md paragraph
    pub(crate) fn scan_interrupt_paragraph(&self) -> bool {
        self.scan_bullet_list().is_some()
            || self.scan_ordered_list().is_some()
            || self.scan_atx_heading().is_some()
            || self.scan_two_newlines()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
pub(crate) struct MacroSpan {
    pub operator: (usize, usize),
    pub name: (usize, usize),
    pub args: (usize, usize),
}
