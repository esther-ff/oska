pub(crate) struct Walker<'w> {
    data: &'w [u8],
    len: usize,
    position: usize,
    last: Option<u8>,
}

impl<'w> Walker<'w> {
    /// Creates a new `Walker`
    /// Verifies if the bytes provided
    /// form a UTF-8 string.
    pub(crate) fn new(data: &'w [u8]) -> Self {
        #[cfg(debug_assertions)]
        let _ = core::str::from_utf8(data).expect("used non-utf8 text");

        Self {
            last: None,
            position: 0,
            len: data.len(),
            data,
        }
    }

    /// Goes one character forward.
    pub(crate) fn next(&mut self) -> Option<u8> {
        if self.position >= self.len {
            return None;
        }

        let val = self.data[self.position];
        self.position += 1;

        self.last = Some(val);

        Some(val)
    }

    /// Goes `teps` of characters back
    pub(crate) fn back(&mut self, steps: usize) -> Option<u8> {
        if (self.position + steps > self.len) | (steps > self.len) {
            return None;
        }

        self.position -= steps;

        Some(self.data[self.position])
    }

    /// Peeks `chars` forward
    /// Note: `peek`ing 0 characters, will give the character
    /// at the current position
    pub(crate) fn peek(&self, chars: usize) -> Option<u8> {
        if (self.position + chars > self.len) | (chars > self.len) {
            return None;
        }

        let val = self.data[self.position + chars];

        Some(val)
    }

    /// Returns the position
    pub(crate) fn position(&self) -> usize {
        self.position
    }

    /// Advances the position by `chars`
    pub(crate) fn advance(&mut self, chars: usize) {
        self.position += chars
    }

    /// Retreats the position by `chars`
    pub(crate) fn retreat(&mut self, chars: usize) {
        self.position -= chars
    }

    /// Sets the position
    pub(crate) fn set_position(&mut self, pos: usize) {
        self.position = pos
    }

    /// Checks if the next char is equal to `target`
    pub(crate) fn is_next_char(&mut self, target: u8) -> bool {
        self.peek(0).map_or(false, |char| char == target)
    }

    /// Executes the given closure, using the next character as an argument
    /// returning a boolean
    /// If it's EOF, returns false anyway
    pub(crate) fn is_next_pred<F>(&mut self, pred: F) -> bool
    where
        F: FnOnce(u8) -> bool,
    {
        self.peek(0).map_or(false, pred)
    }

    /// Goes forward till it hits a character
    /// as in:
    /// ```rust,ignore
    /// use oska::walker::Walker;
    ///
    /// let text = "Haha!";
    /// let mut w = Walker::new(text.as_bytes());
    ///
    /// assert!(w.till(b'!') == Some("Haha"));
    /// ```
    pub(crate) fn till(&mut self, target: u8) -> Option<&str> {
        let start = self.position();
        let mut found = false;

        while let Some(char) = self.next() {
            if char == target {
                found = true;
                break;
            }

            if self.is_next_char(target) {
                found = true;
                break;
            }
        }

        if found {
            let bytes = &self.data[start..self.position()];

            Some(core::str::from_utf8(bytes).expect("invalid utf-8"))
        } else {
            None
        }
    }

    /// Goes forward till it stops finding a character
    /// as in:
    /// ```rust,ignore
    /// use oska::walker::Walker;
    ///
    /// let text = "***A";
    /// let mut w = Walker::new(text.as_bytes());
    ///
    /// assert!(w.till_not(b'*') == 3);
    /// assert!(w.next().unwrap() == b'A');
    /// ```
    fn till_not(&mut self, target: u8) -> usize {
        let mut count = 0;

        while let Some(val) = self.next() {
            if val == target {
                count += 1;
            } else {
                self.retreat(1);
                break;
            }
        }

        count
    }
}

#[cfg(test)]
mod tests {
    use super::Walker;

    #[test]
    fn next() {
        let text = "******";
        let mut w = Walker::new(text.as_bytes());

        while let Some(_) = w.next() {}

        assert!(w.next().is_none());

        assert!(w.position() == 6);
    }

    #[test]
    fn back() {
        let text = "ABC";

        let mut w = Walker::new(text.as_bytes());

        assert!(w.next().unwrap() == b'A');

        assert!(w.next().unwrap() == b'B');

        assert!(w.back(1).unwrap() == b'B');
    }

    #[test]
    fn peek() {
        let text = "ABCDEF";

        let w = Walker::new(text.as_bytes());

        assert!(w.peek(123).is_none());
        assert!(w.peek(5).unwrap() == b'F');
    }

    #[test]
    fn is_next_char() {
        let text = "HAHA";

        let mut w = Walker::new(text.as_bytes());

        assert!(w.next().unwrap() == b'H');

        assert!(w.is_next_char(b'A'));
    }

    #[test]
    fn is_next_pred() {
        let text = "ABC";

        let mut w = Walker::new(text.as_bytes());

        assert!(w.next().unwrap() == b'A');
        dbg!((w.peek(0), b'B'));
        assert!(w.is_next_pred(|char| char == b'B'));
    }

    #[test]
    fn till_not() {
        let text = "!!!!!!";

        let mut w = Walker::new(text.as_bytes());

        assert!(w.till_not(b'!') == 6);
    }

    #[test]
    fn till() {
        let text = "i like cake!";

        let mut w = Walker::new(text.as_bytes());

        assert!(w.till(b'!') == Some("i like cake"));
    }

    #[test]
    fn till_not_and_next() {
        let text = "**Wawa";

        let mut w = Walker::new(text.as_bytes());

        assert!(w.till_not(b'*') == 2);
        assert!(w.next() == Some(b'W'))
    }
}
