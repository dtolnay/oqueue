use super::{Inner, Output};
use crate::sync::Mutex;
use std::fmt::{self, Debug};
use std::io::{Result, Write};
use std::rc::Rc;
use std::sync::Arc;
use termcolor::{ColorSpec, WriteColor};

#[readonly::make]
#[derive(Clone)]
pub struct Task {
    handle: Rc<Handle>,

    #[readonly]
    pub index: usize,
}

struct Handle {
    inner: Arc<Mutex<Inner>>,
    index: usize,
}

impl Debug for Task {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter
            .debug_tuple("Task")
            .field(&self.handle.index)
            .finish()
    }
}

impl Task {
    pub(super) fn new(index: usize, inner: Arc<Mutex<Inner>>) -> Self {
        Task {
            handle: Rc::new(Handle { inner, index }),
            index,
        }
    }

    #[doc(hidden)]
    pub fn write_fmt(&self, args: fmt::Arguments) {
        let _ = self.apply(|w| w.write_fmt(args));
    }

    fn apply<T>(&self, f: impl FnOnce(&mut dyn WriteColor) -> T) -> T {
        let inner = &mut *self.handle.inner.lock();

        if self.handle.index == inner.finished {
            f(&mut inner.stream)
        } else {
            f(&mut inner.get(self.handle.index).buffer)
        }
    }
}

impl Write for Task {
    fn write(&mut self, b: &[u8]) -> Result<usize> {
        self.apply(|w| w.write(b))
    }

    fn flush(&mut self) -> Result<()> {
        self.apply(|w| w.flush())
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.apply(|w| w.write_all(buf))
    }

    fn write_fmt(&mut self, args: fmt::Arguments) -> Result<()> {
        self.apply(|w| w.write_fmt(args))
    }
}

impl WriteColor for Task {
    fn supports_color(&self) -> bool {
        self.apply(|w| w.supports_color())
    }

    fn set_color(&mut self, spec: &ColorSpec) -> Result<()> {
        self.apply(|w| w.set_color(spec))
    }

    fn reset(&mut self) -> Result<()> {
        self.apply(|w| w.reset())
    }
}

impl Drop for Handle {
    fn drop(&mut self) {
        let mut inner = &mut *self.inner.lock();

        inner.get(self.index).done = true;

        while inner.pending.get(0).map_or(false, Output::is_done) {
            inner.finished += 1;
            let mut task = inner.pending.pop_front().unwrap();
            let _ = task.buffer.reset();
            let _ = inner.writer.print(&task.buffer);
        }

        if let Some(head) = inner.pending.get_mut(0) {
            let _ = inner.writer.print(&head.buffer);
            head.buffer.clear();
        }
    }
}
