use core::{cell::UnsafeCell, ops::Deref};

use super::{spinlock::SpinLock, Lock};

enum State<T> {
    Uninit,
    Init(T),
}

impl<T> State<T> {
    #[inline(always)]
    fn unwrap(&self) -> &T {
        match self {
            State::Init(v) => v,
            _ => panic!("Unwrapped empty LazyLock"),
        }
    }
}

pub struct LazyLock<T, L: Lock, F = fn() -> T>
where
    F: FnOnce() -> T,
{
    data: UnsafeCell<State<T>>,
    init: F,
    lock: L,
}

unsafe impl<T, L: Lock> Sync for LazyLock<T, L> {}
unsafe impl<T, L: Lock> Send for LazyLock<T, L> {}

impl<T, F: FnOnce() -> T> LazyLock<T, SpinLock, F> {
    pub const fn new(f: F) -> Self {
        Self {
            data: UnsafeCell::new(State::Uninit),
            init: f,
            lock: SpinLock::new(),
        }
    }
}

impl<T, L> Deref for LazyLock<T, L>
where
    L: Lock,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let guard = self.lock.lock().unwrap();
        let data = unsafe { &*self.data.get() };
        match data {
            State::Uninit => {
                // SAFETY:
                // We have taken out a lock at the beginning of this function,
                // so we know that we're the only one accessing this data.
                // The `data` refence technically observes the change but we
                // happily ignore it, since we know there are no accesses to
                // `data` while we write to it.
                unsafe { self.data.get().write(State::Init((self.init)())) }
                guard.unlock();
                // This unwrap will never panic, as we have just initialsed it.
                data.unwrap()
            }
            State::Init(v) => {
                guard.unlock();
                v
            }
        }
    }
}
