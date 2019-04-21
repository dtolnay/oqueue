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
