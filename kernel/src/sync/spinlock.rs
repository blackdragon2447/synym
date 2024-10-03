use core::sync::atomic::{AtomicBool, Ordering};

use super::{Lock, LockError, LockGuard, LockResult};

pub struct SpinLock {
    lock: AtomicBool,
}

impl SpinLock {
    pub(super) const fn new() -> Self {
        Self {
            lock: AtomicBool::new(false),
        }
    }
}

impl Lock for SpinLock {
    fn lock(&self) -> LockResult<LockGuard<Self>> {
        while self.lock.swap(true, Ordering::SeqCst) {}
        Ok(LockGuard(self))
    }

    fn try_lock(&self) -> LockResult<LockGuard<Self>> {
        if self.lock.swap(true, Ordering::SeqCst) {
            Err(LockError::WouldBlock)
        } else {
            Ok(LockGuard(self))
        }
    }

    fn is_locked(&self) -> bool {
        self.lock.load(Ordering::SeqCst)
    }

    fn unlock(&self) {
        self.lock.store(false, Ordering::SeqCst)
    }
}
