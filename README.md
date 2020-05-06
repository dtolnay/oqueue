Non-interleaving output queue
=============================

[![Build Status](https://img.shields.io/github/workflow/status/dtolnay/oqueue/CI/master)](https://github.com/dtolnay/oqueue/actions?query=branch%3Amaster)
[![Latest Version](https://img.shields.io/crates/v/oqueue.svg)](https://crates.io/crates/oqueue)
[![Rust Documentation](https://img.shields.io/badge/api-rustdoc-blue.svg)](https://docs.rs/oqueue)

Mechanism to prevent interleaving of output of tasks while still allowing an
arbitrary number of tasks to make progress, even ones other than the task
currently printing output.

```toml
[dependencies]
oqueue = "0.1"
```

<br>

## Use case

This crate addresses a seemingly narrow use case, but it is one I have hit in a
couple different circumstances.

Suppose we have some embarrassingly parallel workload where each piece of work
may want to write to stdout/stderr. If we just parallelize this naïvely, output
from different tasks will interleave and end up unreadable. If we make each task
lock the output stream, do its work, and then unlock, we avoid interleaving but
tasks can no longer run in parallel. If we have each task write its output into
a local buffer and print it all atomically at the end, all output is
unnecessarily delayed and the application can feel lurchy and unresponsive
because none of the output is seen in real time.

<br>

## Objective

  - We have an ordered sequence of tasks 0..N.

  - We want to receive all output from task 0, then all output from task 1, etc
    in order. Task output must not interleave with other tasks and must follow
    the task order.

  - We want tasks to execute in parallel.

  - We want all output to be printed as soon as possible, meaning real time for
    exactly one task at a time and deferred until replacement of the realtime
    task for other tasks.

<br>

## Example

This example uses oqueue to sequence the output of worker threads on a Rayon
threadpool.

```rust
use oqueue::{Color::Red, Sequencer, Task};
use rayon::ThreadPoolBuilder;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    // Come up with some work that needs to be performed. Let's pretend to
    // perform work on each file in the current directory.
    let mut files = Vec::new();
    for entry in fs::read_dir(".")? {
        files.push(entry?.path());
    }
    files.sort();

    // Build a thread pool with one thread per cpu.
    let cpus = num_cpus::get();
    let pool = ThreadPoolBuilder::new().num_threads(cpus).build()?;

    // Spin up the right number of worker threads. They will write to stderr.
    let oqueue = Sequencer::stderr();
    pool.scope(|scope| {
        for _ in 0..cpus {
            scope.spawn(|_| worker(&oqueue, &files));
        }
    });

    Ok(())
}

fn worker(oqueue: &Sequencer, inputs: &[PathBuf]) {
    // Perform tasks indicated by the sequencer.
    loop {
        let task = oqueue.begin();
        match inputs.get(task.index) {
            Some(path) => work(task, path),
            None => return,
        }
    }
}

fn work(task: Task, path: &Path) {
    // Produce output by writing to the task.
    write!(task, "evaluating ");
    task.bold();
    writeln!(task, "{}", path.display());

    // Do some expensive work...
    let string = path.to_string_lossy();
    thread::sleep(Duration::from_millis(150 * string.len() as u64));

    // ... which may fail or succeed.
    if string.contains('c') {
        task.bold_color(Red);
        write!(task, "  ERROR");
        task.reset_color();
        writeln!(task, ": path contains the letter 'c'");
    }
}
```

The output of this program is guaranteed to display tasks in the intended
sorted order and non-interleaved. Tasks will make progress in parallel
without needing to wait to perform output. All output will appear the
earliest possible including one task in real time at all times.

<pre>
evaluating <b>./.git</b>
evaluating <b>./.gitignore</b>
evaluating <b>./.travis.yml</b>
evaluating <b>./Cargo.lock</b>
  <b>ERROR</b>: path contains the letter 'c'
evaluating <b>./Cargo.toml</b>
evaluating <b>./LICENSE-APACHE</b>
evaluating <b>./LICENSE-MIT</b>
evaluating <b>./README.md</b>
evaluating <b>./examples</b>
evaluating <b>./src</b>
  <b>ERROR</b>: path contains the letter 'c'
evaluating <b>./target</b>
</pre>

<br>

## Further reading

  - The [`oqueue::Sequencer`][Sequencer] documentation covers some different
    techniques for distributing work items across tasks.

  - The [`oqueue::Task`][Task] documentation shows the APIs for setting
    output color and writing output to a task.

[Sequencer]: https://docs.rs/oqueue/0.1/oqueue/struct.Sequencer.html
[Task]: https://docs.rs/oqueue/0.1/oqueue/struct.Task.html

<br>

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
</sub>
