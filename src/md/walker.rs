pub struct Walker<'w> {
    data: &'w [u8],
    len: usize,
    position: usize,
    last: Option<u8>,
}

#[derive(Debug)]
pub struct StrRange {
    start: usize,
    end: usize,
}

impl StrRange {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn get(&self) -> (usize, usize) {
        (self.start, self.end)
    }

    pub fn resolve(self, data: &[u8]) -> &str {
        let bytes = data
            .get(self.start..self.end)
            .expect("out of bounds access");

        // Safety:
        //
        // The slice and it's bounds point to a valid subslice
        // of utf-8 characters.
        unsafe { core::str::from_utf8_unchecked(bytes) }
    }

    pub fn adjust<F>(&mut self, func: F)
    where
        F: FnOnce((&mut usize, &mut usize)),
    {
        func((&mut self.start, &mut self.end))
    }
}

impl<'w> Walker<'w> {
    /// Creates a new `Walker`
    /// Verifies if the bytes provided
    /// form a UTF-8 string.
    pub(crate) fn new<T: Into<&'w [u8]>>(data: T) -> Self {
        // #[cfg(debug_assertions)]
        // let _ = core::str::from_utf8(data).expect("used non-utf8 text");

        let actual = data.into();
        Self {
            last: None,
            position: 0,
            len: actual.len(),
            data: actual,
        }
    }

    pub(crate) unsafe fn _get_rest_test(&mut self) {
        let text = self.data.get(self.position()..self.len - 1).unwrap();

        println!("{}", core::str::from_utf8(text).unwrap());
    }

    pub(crate) fn get(&self, start: usize, end: usize) -> Option<StrRange> {
        if start > self.len && end < self.len {
            return StrRange::new(start, end).into();
        }

        None
    }

    pub(crate) fn data(&self) -> &[u8] {
        self.data
    }

    pub(crate) fn data_from_offset(&self, initial: usize) -> &[u8] {
        debug_assert!(
            self.position() <= self.data().len(),
            "position of cursor is further than the data's length"
        );

        dbg!(initial);
        dbg!(self.position());
        debug_assert!(
            initial <= self.position(),
            "offset is bigger than the current position"
        );

        unsafe { self.data().get_unchecked(initial..self.position()) }
    }

    pub(crate) fn walker_from_offset(&self, initial: usize) -> Walker<'_> {
        let data = self.data_from_offset(initial);

        Walker::new(data)
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
        self.data.get(self.position() + chars).copied()
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
        self.peek(0) == Some(target)
    }

    /// Executes the given closure, using the next character as an argument
    /// returning a boolean
    /// If it's EOF, returns false anyway
    pub(crate) fn is_next_pred<F>(&mut self, pred: F) -> bool
    where
        F: FnOnce(u8) -> bool,
    {
        self.peek(0).is_some_and(pred)
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
    pub(crate) fn till(&mut self, target: u8) -> Option<StrRange> {
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
            let bytes = StrRange::new(start, self.position());

            Some(bytes)
        } else {
            None
        }
    }

    /// Goes forward till it hits a character
    /// doesn't care if it doesn't find the actual target
    /// as in:
    /// ```rust,ignore
    /// use oska::walker::Walker;
    ///
    /// let text = "Haha!";
    /// let mut w = Walker::new(text.as_bytes());
    ///
    /// assert!(w.till(b'!') == Some("Haha"));
    /// ```
    pub(crate) fn till_inclusive(&mut self, target: u8) -> StrRange {
        let start = self.position();

        while let Some(char) = self.next() {
            if char == target {
                break;
            }

            if self.is_next_char(target) {
                break;
            }
        }

        StrRange::new(start, self.position())
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
    pub(crate) fn till_not(&mut self, target: u8) -> usize {
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

        let string = w.till(b'!').unwrap().resolve(text.as_bytes());

        assert!(string == "i like cake");
    }

    #[test]
    fn till_not_and_next() {
        let text = "**Wawa";

        let mut w = Walker::new(text.as_bytes());

        assert!(w.till_not(b'*') == 2);
        assert!(w.next() == Some(b'W'))
    }
}
