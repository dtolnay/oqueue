#![cfg_attr(oqueue_doc_cfg, feature(doc_cfg))]

mod sequencer;
mod sync;

pub use crate::sequencer::{Sequencer, Task};
