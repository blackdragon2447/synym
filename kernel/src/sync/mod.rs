#![allow(private_bounds)]

mod lazylock;
mod mutex;
mod spinlock;

pub use lazylock::LazyLock;
pub use mutex::Mutex;
pub use spinlock::SpinLock;

#[derive(Debug)]
pub enum LockError {
    Poisoned,
    WouldBlock,
}

pub type LockResult<T> = Result<T, LockError>;

struct LockGuard<'l, L: Lock>(&'l L);

impl<L: Lock> LockGuard<'_, L> {
    fn unlock(self) {
        // Self moved into unlock and then drop, drop causes the unlock.
    }
}

impl<L: Lock> Drop for LockGuard<'_, L> {
    fn drop(&mut self) {
        self.0.unlock();
    }
}

trait Lock
where
    Self: Sized,
{
    fn lock(&self) -> LockResult<LockGuard<Self>>;

    fn try_lock(&self) -> LockResult<LockGuard<Self>>;

    fn is_locked(&self) -> bool;

    fn unlock(&self);
}
