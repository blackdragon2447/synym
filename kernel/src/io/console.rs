use core::fmt::{Debug, Display};

#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            writeln!($crate::sbi::debug_console::Console, $($arg)*).unwrap()
        }
    };
    () => {
        {
            use core::fmt::Write;
            writeln!($crate::sbi::debug_console::Console, "").unwrap()
        }
    };
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            write!($crate::sbi::debug_console::Console, $($arg)*).unwrap()
        }
    };
    () => {
        {
            use core::fmt::Write;
            write!($crate::sbi::debug_console::Console, "").unwrap()
        }
    };
}

pub struct Indent(pub usize);

impl Display for Indent {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for _ in 0..self.0 {
            write!(f, "\t")?;
        }
        Ok(())
    }
}

pub struct DebugLimList<'a, T: Debug>(pub &'a [T]);

impl<'a, T: Debug> Debug for DebugLimList<'a, T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.0.len() <= 5 {
            f.debug_list().entries(self.0.iter()).finish()
        } else {
            f.debug_list()
                .entries(self.0.iter().take(5))
                .finish_non_exhaustive()
        }
    }
}
