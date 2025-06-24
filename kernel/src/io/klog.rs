use core::sync::atomic::{AtomicU8, Ordering};

static LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::Info as u8);

#[derive(PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum LogLevel {
    Trace = 5,
    Debug = 4,
    Info = 3,
    Warn = 2,
    Error = 1,
}

pub fn set_loglevel(level: LogLevel) {
    LOG_LEVEL.store(level as u8, Ordering::Relaxed);
}

pub fn get_loglevel() -> LogLevel {
    // Hack, but safe because LOG_LEVEL can only be set in set_loglevel, and that always set a
    // valid value for LogLevel
    unsafe { core::mem::transmute::<u8, LogLevel>(LOG_LEVEL.load(Ordering::Relaxed)) }
}

#[macro_export]
macro_rules! trace {
    ($($arg:tt)*) => {
        if $crate::io::klog::get_loglevel() >= $crate::io::klog::LogLevel::Trace {
            use core::fmt::Write;
            write!($crate::sbi::debug_console::Console, "[TRACE {:>20}:{:<3}] ", file!(), line!()).unwrap();
            writeln!($crate::sbi::debug_console::Console, $($arg)*).unwrap();
        }
    };
}

#[macro_export]
macro_rules! debug {
    ($($arg:tt)*) => {
        if $crate::io::klog::get_loglevel() >= $crate::io::klog::LogLevel::Debug {
            use core::fmt::Write;
            write!($crate::sbi::debug_console::Console, "[DEBUG {:>20}:{:<3}] ", file!(), line!()).unwrap();
            writeln!($crate::sbi::debug_console::Console, $($arg)*).unwrap();
        }
    };
}

#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        if $crate::io::klog::get_loglevel() >= $crate::io::klog::LogLevel::Info {
            use core::fmt::Write;
            write!($crate::sbi::debug_console::Console, "[INFO  {:>20}:{:<3}] ", file!(), line!()).unwrap();
            writeln!($crate::sbi::debug_console::Console, $($arg)*).unwrap();
        }
    };
}

#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {
        if $crate::io::klog::get_loglevel() >= $crate::io::klog::LogLevel::Warn {
            use core::fmt::Write;
            write!($crate::sbi::debug_console::Console, "[WARN  {:>20}:{:<3}] ", file!(), line!()).unwrap();
            writeln!($crate::sbi::debug_console::Console, $($arg)*).unwrap();
        }
    };
}

#[macro_export]
macro_rules! error {
    ($($arg:tt)*) => {
        if $crate::io::klog::get_loglevel() >= $crate::io::klog::LogLevel::Error {
            use core::fmt::Write;
            write!($crate::sbi::debug_console::Console, "[ERROR {:>20}:{:<3}] ", file!(), line!()).unwrap();
            writeln!($crate::sbi::debug_console::Console, $($arg)*).unwrap();
        }
    };
}
