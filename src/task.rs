use super::{Inner, Output};
use crate::sync::Mutex;
use std::fmt::{self, Debug};
use std::io::{Result, Write};
use std::rc::Rc;
use std::sync::Arc;
use termcolor::{Color, ColorSpec, WriteColor};

/// Unit of work arranged by a Sequencer.
///
/// Use the standard library `write!` or `writeln!` macros for writing the
/// output of a task. Additionally this type provides some methods for setting
/// the color of task output.
///
/// Refer to the crate-level documentation and the documentation of the
/// Sequencer type for the recommended patterns of launching tasks.
///
/// ```
/// use oqueue::{Color::Blue, Task};
///
/// fn work(task: Task) {
///     task.color(Blue);
///     writeln!(task, "hello from task #{}", task.index);
/// }
/// ```
#[readonly::make]
#[derive(Clone)]
pub struct Task {
    handle: Rc<Handle>,

    /// Index of the current task. This is a sequential counter that begins at 0
    /// and increments by 1 for each successively started task. It may be
    /// helpful in determining what work this task is responsible for
    /// performing.
    ///
    /// This field is read-only; writing to its value will not compile.
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

    /// Set output to appear in bold uncolored.
    pub fn bold(&self) {
        let mut spec = ColorSpec::new();
        spec.set_bold(true);
        let _ = self.apply(|w| w.set_color(&spec));
    }

    /// Set output to appear in color (not bold).
    pub fn color(&self, color: Color) {
        let mut spec = ColorSpec::new();
        spec.set_fg(Some(color));
        let _ = self.apply(|w| w.set_color(&spec));
    }

    /// Set output to appear bold and colored.
    pub fn bold_color(&self, color: Color) {
        let mut spec = ColorSpec::new();
        spec.set_bold(true);
        spec.set_fg(Some(color));
        let _ = self.apply(|w| w.set_color(&spec));
    }

    /// Set output to non-bold uncolored.
    pub fn reset_color(&self) {
        let _ = self.apply(|w| w.reset());
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
