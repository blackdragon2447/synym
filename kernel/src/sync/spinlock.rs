use core::sync::atomic::{AtomicBool, Ordering};

use super::{Lock, LockError, LockGuard, LockResult, Unlock};

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
        while self.lock.swap(true, Ordering::Acquire) {}
        Ok(LockGuard(self))
    }

    fn try_lock(&self) -> LockResult<LockGuard<Self>> {
        if self.lock.swap(true, Ordering::Acquire) {
            Err(LockError::WouldBlock)
        } else {
            Ok(LockGuard(self))
        }
    }

    fn is_locked(&self) -> bool {
        self.lock.load(Ordering::Relaxed)
    }
}

impl Unlock for SpinLock {
    fn unlock(&self) {
        self.lock.store(false, Ordering::Release)
    }
}
