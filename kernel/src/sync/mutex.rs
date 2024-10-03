use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
};

use super::{spinlock::SpinLock, Lock, LockGuard, LockResult};

pub struct Mutex<T, L: Lock> {
    data: UnsafeCell<T>,
    lock: L,
}

pub struct MutexGuard<'a, T, L: Lock> {
    guard: LockGuard<'a, L>,
    data: &'a mut T,
}

impl<T> Mutex<T, SpinLock> {
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
            lock: SpinLock::new(),
        }
    }
}

unsafe impl<T, L: Lock> Send for Mutex<T, L> {}
unsafe impl<T, L: Lock> Sync for Mutex<T, L> {}

impl<T, L: Lock> Mutex<T, L> {
    pub fn lock(&self) -> LockResult<MutexGuard<T, L>> {
        let guard = self.lock.lock()?;
        Ok(MutexGuard {
            guard,
            data: unsafe { &mut *self.data.get() },
        })
    }

    pub fn try_lock(&self) -> LockResult<MutexGuard<T, L>> {
        let guard = self.lock.try_lock()?;
        Ok(MutexGuard {
            guard,
            data: unsafe { &mut *self.data.get() },
        })
    }
}

impl<T, L: Lock> MutexGuard<'_, T, L> {
    pub fn unlock(self) {
        self.guard.unlock()
    }
}

impl<T, L: Lock> Deref for MutexGuard<'_, T, L> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.data
    }
}

impl<T, L: Lock> DerefMut for MutexGuard<'_, T, L> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.data
    }
}
