use core::{
    mem,
    sync::atomic::{AtomicU8, Ordering},
};

pub struct Once {
    status: AtomicStatus,
}

impl Once {
    pub const fn new() -> Self {
        Self {
            status: AtomicStatus::new(),
        }
    }

    pub fn call_once<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        if self.status.is_completed() {
            return;
        }

        self.call_once_slow(f)
    }

    fn call_once_slow<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        loop {
            let status = self.status.compare_and_swap(
                Status::Uninit,
                Status::Running,
                Ordering::Acquire,
                Ordering::Acquire,
            );

            match status {
                Ok(Status::Uninit) => {
                    let guard = FinishGuard(&self.status);
                    f();
                    mem::forget(guard);
                    self.status.store(Status::Completed, Ordering::Release);
                    return;
                }
                Ok(_) | Err(Status::Uninit) => unreachable!(),
                Err(Status::Running) => continue,
                Err(Status::Completed) => return,
                Err(Status::Panicked) => panic!("Once panicked"),
            }
        }
    }

    pub fn call_once_force<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        if self.status.is_completed() {
            return;
        }

        self.call_once_force_slow(f)
    }

    fn call_once_force_slow<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        loop {
            let status = self.status.compare_and_swap(
                Status::Uninit,
                Status::Running,
                Ordering::Acquire,
                Ordering::Acquire,
            );

            match status {
                Ok(Status::Uninit) => {
                    let guard = FinishGuard(&self.status);
                    f();
                    mem::forget(guard);
                    self.status.store(Status::Completed, Ordering::Release);
                    return;
                }
                Ok(_) | Err(Status::Uninit) => unreachable!(),
                Err(Status::Running) => continue,
                Err(Status::Completed) => return,
                Err(Status::Panicked) => {}
            }

            let status = self.status.compare_and_swap(
                Status::Panicked,
                Status::Running,
                Ordering::Acquire,
                Ordering::Acquire,
            );

            match status {
                Ok(Status::Panicked) => {
                    let guard = FinishGuard(&self.status);
                    f();
                    mem::forget(guard);
                    self.status.store(Status::Completed, Ordering::Release);
                    return;
                }
                Ok(_) | Err(Status::Panicked) => unreachable!(),
                Err(Status::Running) => continue,
                Err(Status::Completed) => return,
                Err(Status::Uninit) => unreachable!(),
            }
        }
    }

    pub fn is_completed(&self) -> bool {
        self.status.is_completed()
    }

    pub fn wait(&self) {
        loop {
            match self.status.load(Ordering::Relaxed) {
                Status::Uninit | Status::Running => continue,
                Status::Completed => return,
                Status::Panicked => panic!("Once panicked"),
            }
        }
    }

    pub fn wait_force(&self) {
        loop {
            match self.status.load(Ordering::Relaxed) {
                Status::Uninit | Status::Running | Status::Panicked => continue,
                Status::Completed => return,
            }
        }
    }
}

#[repr(transparent)]
struct AtomicStatus(AtomicU8);

impl AtomicStatus {
    const fn new() -> Self {
        Self(AtomicU8::new(Status::Uninit as u8))
    }

    fn load(&self, ordering: Ordering) -> Status {
        unsafe { Status::new_unchecked(self.0.load(ordering)) }
    }

    fn store(&self, status: Status, ordering: Ordering) {
        self.0.store(status as u8, ordering);
    }

    fn is_completed(&self) -> bool {
        self.load(Ordering::Acquire) == Status::Completed
    }

    fn compare_and_swap(
        &self,
        old: Status,
        new: Status,
        success: Ordering,
        failure: Ordering,
    ) -> Result<Status, Status> {
        self.0
            .compare_exchange(old as u8, new as u8, success, failure)
            .map(|v| unsafe { Status::new_unchecked(v) })
            .map_err(|v| unsafe { Status::new_unchecked(v) })
    }
}

#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Uninit,
    Running,
    Completed,
    Panicked,
}

impl Status {
    unsafe fn new_unchecked(val: u8) -> Self {
        mem::transmute(val)
    }
}

struct FinishGuard<'a>(&'a AtomicStatus);

impl Drop for FinishGuard<'_> {
    fn drop(&mut self) {
        self.0.store(Status::Panicked, Ordering::SeqCst);
    }
}
