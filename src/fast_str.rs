use core::{alloc, marker::PhantomData};
use std::{
    alloc::{Layout, realloc},
    ptr::NonNull,
};

const FASTSTR_BUFFER_PREALLOCATION: usize = 128;

struct CapacityLen {
    num: usize,
}

impl CapacityLen {
    const LOW: usize = usize::pow(2, (size_of::<usize>() / 2) as u32) - 1;
    const HIGH: usize = !Self::LOW;

    pub fn new() -> Self {
        Self { num: 0 }
    }

    pub fn len(&self) -> usize {
        self.num & Self::LOW
    }

    pub fn cap(&self) -> usize {
        self.num & Self::HIGH
    }

    pub fn len_mut<F: FnOnce(usize) -> usize>(&mut self, f: F) {
        let num = self.len();
        self.num = self.cap() + f(num)
    }

    pub fn cap_mut<F: FnOnce(usize) -> usize>(&mut self, f: F) {
        let num = self.cap();
        self.num = f(num) + self.len();
    }
}

struct HeapBuf {
    ptr: *mut u8,
    sizes: CapacityLen,
}

impl HeapBuf {
    const ALLOC_LAYOUT: Layout = Layout::new::<u8>();

    pub fn new() -> Self {
        Self {
            ptr: core::ptr::null_mut(),
            sizes: CapacityLen::new(),
        }
    }

    pub fn slice<'a>(&'a self) -> &'a [u8] {
        let ptr = self.ptr as *const u8;
        unsafe { core::slice::from_raw_parts(ptr, self.len()) }
    }

    pub fn len(&self) -> usize {
        self.sizes.len()
    }

    pub fn cap(&self) -> usize {
        self.sizes.cap()
    }

    fn grow(&mut self, size: usize) {
        assert!(size <= isize::MAX as usize);

        let ptr = unsafe { realloc(self.ptr, Self::ALLOC_LAYOUT, size) };

        self.ptr = ptr;

        self.sizes.len_mut(|x| x + size);
        self.sizes.cap_mut(|x| x + size);
    }

    pub unsafe fn push_to<'a, A: Into<&'a [u8]>>(&mut self, content: A, len_content: usize) {
        if len_content == 0 {
            return;
        }

        let old_len = self.len();

        if len_content + old_len > self.cap() {
            self.grow(len_content);
        }

        content
            .into()
            .into_iter()
            .enumerate()
            .for_each(|(index, val)| unsafe { self.ptr.add(old_len + index).write(*val) });
    }
}

fn push_str_stack(accum: &mut FastStr, val: &[u8]) {
    if val.len() > FASTSTR_BUFFER_PREALLOCATION - 1
        || val.len() > FASTSTR_BUFFER_PREALLOCATION - accum.cur as usize
    {
        accum.vtable.switch_to_heap(&mut accum.heap_buf);

        unsafe {
            (accum.vtable.push_str)(accum, val);
        }

        return;
    }

    let amnt = core::cmp::min(val.len(), accum.buf.len());

    if amnt == 1 {
        accum.buf[0] = val[0]
    } else {
        accum.buf[..amnt].copy_from_slice(val);
    }

    // amnt will always be below threshold for cut off
    // because of static stack array size
    accum.cur += amnt as u8
}

unsafe fn push_str_heap(accum: &mut FastStr, val: &[u8]) {
    unsafe {
        accum
            .heap_buf
            .as_mut()
            .expect("expected HeapBuf")
            .push_to(val, val.len())
    }
}

unsafe fn get_string_heap(accum: &mut FastStr) -> &str {
    let buf = accum.heap_buf.as_mut().expect("i expect this here!");

    core::str::from_utf8(buf.slice()).expect("invalid utf-8")
}

struct Vtable {
    push_str: unsafe fn(&mut FastStr, &[u8]),
}

impl Vtable {
    pub fn new() -> Self {
        Vtable {
            push_str: push_str_stack,
        }
    }

    pub fn switch_to_heap(&mut self, heap: &mut Option<HeapBuf>) {
        debug_assert!(heap.is_none());

        *heap = Some(HeapBuf::new());
        self.push_str = push_str_heap;
    }
}

pub(crate) struct FastStr {
    buf: [u8; FASTSTR_BUFFER_PREALLOCATION],
    cur: u8,
    heap_buf: Option<HeapBuf>,
    vtable: Vtable,
}

impl FastStr {
    pub fn new() -> Self {
        Self {
            buf: [0; FASTSTR_BUFFER_PREALLOCATION],
            cur: 0,

            heap_buf: None,

            vtable: Vtable::new(),
        }
    }

    pub fn push_str<'a, A: Into<&'a [u8]>>(&'a mut self, val: A) {
        unsafe {
            (self.vtable.push_str)(self, val.into());
        }
    }

    pub fn show(&self) -> &str {
        todo!()
    }
}
