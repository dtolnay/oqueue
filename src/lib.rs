//! Mechanism to prevent interleaving of output of tasks while still allowing an
//! arbitrary number of tasks to make progress, even ones other than the task
//! currently printing output.
//!
//! # Use case
//!
//! This crate addresses a seemingly narrow use case, but it is one I have hit
//! in a couple different circumstances.
//!
//! Suppose we have some embarrassingly parallel workload where each piece of
//! work may want to write to stdout/stderr. If we just parallelize this
//! naïvely, output from different tasks will interleave and end up unreadable.
//! If we make each task lock the output stream, do its work, and then unlock,
//! we avoid interleaving but tasks can no longer run in parallel. If we have
//! each task write its output into a local buffer and print it all atomically
//! at the end, all output is unnecessarily delayed and the application can feel
//! lurchy and unresponsive because none of the output is seen in real time.
//!
//! # Objective
//!
//!   - We have an ordered sequence of tasks 0..N.
//!
//!   - We want to receive all output from task 0, then all output from task 1,
//!     etc in order. Task output must not interleave with other tasks and must
//!     follow the task order.
//!
//!   - We want tasks to execute in parallel.
//!
//!   - We want all output to be printed as soon as possible, meaning real time
//!     for exactly one task at a time and deferred until replacement of the
//!     realtime task for other tasks.
//!

#![cfg_attr(oqueue_doc_cfg, feature(doc_cfg))]

mod sequencer;
mod sync;

pub use crate::sequencer::{Sequencer, Task};

#[doc(no_inline)]
pub use termcolor::Color;
