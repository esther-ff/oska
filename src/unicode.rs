use unicode_segmentation::GraphemeCursor;

pub(crate) fn utf8<'a>(str: &'a str, ext: bool) -> Utf8<'a> {
    Utf8 {
        str,
        cur: GraphemeCursor::new(0, str.len(), ext),
    }
}

pub(crate) struct Utf8<'u> {
    str: &'u str,
    cur: GraphemeCursor,
}

impl<'u, 'c> Utf8<'u>
where
    'u: 'c,
{
    pub fn pos(&self) -> usize {
        self.cur.cur_cursor()
    }

    pub fn get_from_str(&'c self, left: usize, right: usize) -> Option<&'u str> {
        let rf = match self.str.get(left..right) {
            None => return None,

            Some(val) => unsafe { &*(val as *const str) },
        };

        Some(rf)
    }

    pub fn len(&self) -> usize {
        self.str.len()
    }

    pub fn back(&'c mut self) -> Option<&'u str> {
        let current = self.cur.cur_cursor();
        match self.cur.prev_boundary(&self.str, 0) {
            Ok(offset) => match offset {
                Some(num) => {
                    self.cur.set_cursor(num);
                    self.str.get(current..num).as_ref().map(|x| *x)
                }
                None => return None,
            },

            Err(_graph_err) => {
                todo!()
            }
        }
    }

    pub fn eat(&mut self) {
        let _ = self.cur.next_boundary(&self.str, 0);
    }

    pub fn count_chars(&mut self, target: &str) -> usize {
        let mut count = 0;
        loop {
            match self.next() {
                None => return count,

                Some(found) => {
                    if found != target {
                        return count;
                    } else {
                        count += 1;

                        // rewrite...
                        if let Some(val) = self.peek() {
                            if val != target {
                                return count;
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn peek(&'c mut self) -> Option<&'u str> {
        let current = self.cur.cur_cursor();
        match self.cur.next_boundary(&self.str, 0) {
            Ok(offset) => match offset {
                Some(num) => {
                    let string = self.str.get(current..num).as_ref().copied();

                    let _ = self.cur.prev_boundary(&self.str, 0);

                    string
                }
                None => return None,
            },

            Err(_graph_err) => {
                unimplemented!()
            }
        }
    }

    pub fn peek2(&'c mut self) -> Option<&'u str> {
        self.next();
        let val = self.peek();

        self.back();

        val
    }
}

impl<'u> Iterator for Utf8<'u> {
    type Item = &'u str;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.cur.cur_cursor();
        match self.cur.next_boundary(&self.str, 0) {
            Ok(offset) => match offset {
                Some(num) => self.str.get(current..num).as_ref().copied(),
                None => return None,
            },

            Err(_graph_err) => {
                unimplemented!()
            }
        }
    }
}
