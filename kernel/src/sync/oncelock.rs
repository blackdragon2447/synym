use core::{
    cell::UnsafeCell,
    mem::{self, MaybeUninit},
};

use super::Once;

pub struct OnceLock<T> {
    data: UnsafeCell<MaybeUninit<T>>,
    once: Once,
}

unsafe impl<T> Send for OnceLock<T> {}
unsafe impl<T> Sync for OnceLock<T> {}

impl<T> OnceLock<T> {
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new(MaybeUninit::uninit()),
            once: Once::new(),
        }
    }

    pub fn get(&self) -> Option<&T> {
        if self.once.is_completed() {
            Some(unsafe { self.data.as_ref_unchecked().assume_init_ref() })
        } else {
            None
        }
    }

    pub fn get_mut(&mut self) -> Option<&mut T> {
        if self.once.is_completed() {
            Some(unsafe { self.data.as_mut_unchecked().assume_init_mut() })
        } else {
            None
        }
    }

    pub fn get_or_init<F>(&self, f: F) -> &T
    where
        F: FnOnce() -> T,
    {
        match self.get() {
            Some(t) => t,
            None => {
                self.initialize(f);
                unsafe { self.data.as_ref_unchecked().assume_init_ref() }
            }
        }
    }

    pub fn set(&self, t: T) -> Result<(), T> {
        match self.get() {
            Some(_) => Err(t),
            None => {
                self.initialize(|| t);
                Ok(())
            }
        }
    }

    pub fn take(&mut self) -> Option<T> {
        if self.once.is_completed() {
            let res = unsafe {
                mem::replace(self.data.as_mut_unchecked(), MaybeUninit::uninit()).assume_init()
            };
            // Replace the once to get one that hasn't been ran yet
            self.once = Once::new();
            Some(res)
        } else {
            None
        }
    }

    pub fn wait(&self) -> &T {
        self.once.wait();
        self.get().unwrap()
    }

    fn initialize<F>(&self, f: F)
    where
        F: FnOnce() -> T,
    {
        self.once.call_once(|| {
            let res = f();
            unsafe {
                self.data.as_mut_unchecked().write(res);
            }
        });
    }
}
