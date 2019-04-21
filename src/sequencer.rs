// Clippy false positive:
// https://github.com/rust-lang/rust-clippy/issues/3071
#![allow(clippy::redundant_closure)]

#[path = "task.rs"]
mod task;

use crate::sync::Mutex;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use termcolor::ColorChoice::Auto;
use termcolor::{Buffer, BufferWriter, StandardStream};

pub use self::task::Task;

pub struct Sequencer {
    inner: Arc<Mutex<Inner>>,
    /// Index of next started task.
    started: AtomicUsize,
}

#[cfg(test)]
struct _Test
where
    Sequencer: Send + Sync;

struct Inner {
    stream: StandardStream,
    writer: BufferWriter,
    /// Number of tasks popped from queue.
    finished: usize,
    pending: VecDeque<Output>,
}

struct Output {
    buffer: Buffer,
    done: bool,
}

impl Sequencer {
    fn new(stream: StandardStream, writer: BufferWriter) -> Self {
        Sequencer {
            inner: Arc::new(Mutex::new(Inner {
                stream,
                writer,
                finished: 0,
                pending: VecDeque::new(),
            })),
            started: AtomicUsize::new(0),
        }
    }

    pub fn stdout() -> Self {
        Self::new(StandardStream::stdout(Auto), BufferWriter::stdout(Auto))
    }

    pub fn stderr() -> Self {
        Self::new(StandardStream::stderr(Auto), BufferWriter::stderr(Auto))
    }

    pub fn begin(&self) -> Task {
        let index = self.started.fetch_add(1, Ordering::Relaxed);
        Task::new(index, self.inner.clone())
    }
}

impl Inner {
    fn get(&mut self, index: usize) -> &mut Output {
        assert!(index >= self.finished);
        let offset = index - self.finished;

        if offset >= self.pending.len() {
            let writer = &self.writer;
            self.pending.resize_with(offset + 1, || Output {
                buffer: writer.buffer(),
                done: false,
            });
        }

        &mut self.pending[offset]
    }
}

impl Output {
    fn is_done(&self) -> bool {
        self.done
    }
}
