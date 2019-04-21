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

/// Synchronization mechanism for performing non-interleaved output from
/// concurrent tasks.
///
/// # Bare-bones skeleton
///
/// This example performs 30 tasks in parallel across a pool of 10 threads. Each
/// task writes one line of output. All output is guaranteed to appear in order
/// by task index from 0 through 29.
///
/// ```
/// use oqueue::Sequencer;
///
/// fn main() {
///     let oqueue = Sequencer::stderr();
///
///     // Launch 10 worker threads.
///     rayon::scope(|scope| {
///         for _ in 0..10 {
///             scope.spawn(|_| worker(&oqueue));
///         }
///     });
/// }
///
/// fn worker(oqueue: &Sequencer) {
///     loop {
///         let task = oqueue.begin();
///         if task.index >= 30 {
///             return;
///         }
///         writeln!(task, "hello from task #{}", task.index);
///     }
/// }
/// ```
///
/// <details>
/// <summary style="padding-left:3em"><a><em>▷&emsp;Click to show output</em></a></summary>
///
/// ```text
/// hello from task #0
/// hello from task #1
/// hello from task #2
/// hello from task #3
/// hello from task #4
/// hello from task #5
/// hello from task #6
/// hello from task #7
/// hello from task #8
/// hello from task #9
/// hello from task #10
/// hello from task #11
/// hello from task #12
/// hello from task #13
/// hello from task #14
/// hello from task #15
/// hello from task #16
/// hello from task #17
/// hello from task #18
/// hello from task #19
/// hello from task #20
/// hello from task #21
/// hello from task #22
/// hello from task #23
/// hello from task #24
/// hello from task #25
/// hello from task #26
/// hello from task #27
/// hello from task #28
/// hello from task #29
/// ```
/// </details>
///
/// <br>
///
/// # Shared slice skeleton
///
/// This example uses a shared slice to coordinate work that needs to be
/// performed. Tasks perform work on one element of the slice according to their
/// task index.
///
/// ```
/// use oqueue::Sequencer;
///
/// struct WorkItem(u8);
///
/// fn main() {
///     let oqueue = Sequencer::stderr();
///     let work = (b'A'..=b'Z').map(WorkItem).collect::<Vec<_>>();
///
///     // Launch 10 worker threads.
///     rayon::scope(|scope| {
///         for i in 0..10 {
///             let oqueue = &oqueue;
///             let work = &work;
///             scope.spawn(move |_| worker(i, oqueue, work));
///         }
///     });
/// }
///
/// fn worker(thread: usize, oqueue: &Sequencer, work: &[WorkItem]) {
///     loop {
///         let task = oqueue.begin();
///         let input = match work.get(task.index) {
///             Some(input) => input,
///             None => return,
///         };
///         writeln!(
///             task,
///             "thread {} is performing work {}",
///             thread, input.0 as char,
///         );
///     }
/// }
/// ```
///
/// <details>
/// <summary style="padding-left:3em"><a><em>▷&emsp;Click to show output</em></a></summary>
///
/// ```text
/// thread 0 is performing work A
/// thread 9 is performing work B
/// thread 1 is performing work C
/// thread 2 is performing work D
/// thread 0 is performing work E
/// thread 0 is performing work F
/// thread 0 is performing work G
/// thread 0 is performing work H
/// thread 0 is performing work I
/// thread 0 is performing work J
/// thread 2 is performing work K
/// thread 9 is performing work L
/// thread 9 is performing work M
/// thread 9 is performing work N
/// thread 9 is performing work O
/// thread 9 is performing work P
/// thread 9 is performing work Q
/// thread 9 is performing work R
/// thread 0 is performing work S
/// thread 0 is performing work T
/// thread 0 is performing work U
/// thread 0 is performing work V
/// thread 2 is performing work W
/// thread 1 is performing work X
/// thread 1 is performing work Y
/// thread 1 is performing work Z
/// ```
/// </details>
///
/// <br>
///
/// # Synchronized queue skeleton
///
/// This example uses a synchronized queue of work items in the form of a mutex
/// holding an iterator, although any other channel-like implementation could
/// work too. The task index is not used in this approach.
///
/// ```
/// use oqueue::Sequencer;
/// use std::sync::Mutex;
///
/// struct WorkItem(u8);
///
/// fn main() {
///     let oqueue = Sequencer::stderr();
///     let work = Mutex::new((b'A'..=b'Z').map(WorkItem));
///
///     // Launch 10 worker threads.
///     rayon::scope(|scope| {
///         for i in 0..10 {
///             let oqueue = &oqueue;
///             let work = &work;
///             scope.spawn(move |_| worker(i, oqueue, work));
///         }
///     });
/// }
///
/// fn worker(thread: usize, oqueue: &Sequencer, work: &Mutex<dyn Iterator<Item = WorkItem>>) {
///     loop {
///         let task = oqueue.begin();
///         let input = match work.lock().unwrap().next() {
///             Some(input) => input,
///             None => return,
///         };
///         writeln!(
///             task,
///             "thread {} is performing work {}",
///             thread, input.0 as char,
///         );
///     }
/// }
/// ```
///
/// <details>
/// <summary style="padding-left:3em"><a><em>▷&emsp;Click to show output</em></a></summary>
///
/// ```text
/// thread 9 is performing work A
/// thread 0 is performing work B
/// thread 9 is performing work C
/// thread 9 is performing work D
/// thread 9 is performing work E
/// thread 9 is performing work F
/// thread 9 is performing work G
/// thread 9 is performing work H
/// thread 9 is performing work I
/// thread 1 is performing work J
/// thread 9 is performing work K
/// thread 2 is performing work L
/// thread 1 is performing work M
/// thread 2 is performing work N
/// thread 0 is performing work O
/// thread 9 is performing work P
/// thread 1 is performing work Q
/// thread 1 is performing work R
/// thread 1 is performing work S
/// thread 0 is performing work T
/// thread 1 is performing work U
/// thread 2 is performing work V
/// thread 9 is performing work W
/// thread 0 is performing work X
/// thread 1 is performing work Y
/// thread 1 is performing work Z
/// ```
/// </details>
///
/// <br>
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

    /// Makes a sequencer whose output goes to stdout.
    pub fn stdout() -> Self {
        Self::new(StandardStream::stdout(Auto), BufferWriter::stdout(Auto))
    }

    /// Makes a sequencer whose output goes to stderr.
    pub fn stderr() -> Self {
        Self::new(StandardStream::stderr(Auto), BufferWriter::stderr(Auto))
    }

    /// Begins the next available task.
    ///
    /// The caller may figure out what work to perform based on the index of
    /// this task available in `task.index`, or by acquiring work from a
    /// synchronized queue that is shared across workers.
    ///
    /// This call does not block.
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
