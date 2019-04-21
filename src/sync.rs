use std::sync::{Mutex as StdMutex, MutexGuard, PoisonError};

/// Non-poisoning mutex.
pub(crate) struct Mutex<T: ?Sized> {
    std: StdMutex<T>,
}

impl<T> Mutex<T> {
    pub(crate) fn new(value: T) -> Self {
        Mutex {
            std: StdMutex::new(value),
        }
    }
}

impl<T: ?Sized> Mutex<T> {
    pub(crate) fn lock(&self) -> MutexGuard<T> {
        self.std.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
