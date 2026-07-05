use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::{LockResult, Mutex, MutexGuard},
};

pub(crate) fn guard<T>(fallback: T, body: impl FnOnce() -> T) -> T {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or(fallback)
}

pub(crate) fn guard_void(body: impl FnOnce()) {
    let _ = catch_unwind(AssertUnwindSafe(body));
}

pub(crate) struct RecoveringMutex<T> {
    inner: Mutex<T>,
}

impl<T> RecoveringMutex<T> {
    pub(crate) fn new(value: T) -> Self {
        Self {
            inner: Mutex::new(value),
        }
    }

    pub(crate) fn lock(&self) -> RecoveringLockResult<'_, T> {
        RecoveringLockResult {
            result: self.inner.lock(),
        }
    }
}

pub(crate) struct RecoveringLockResult<'a, T> {
    result: LockResult<MutexGuard<'a, T>>,
}

impl<'a, T> RecoveringLockResult<'a, T> {
    pub(crate) fn expect(self, _message: &str) -> MutexGuard<'a, T> {
        self.result.unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
