use core::arch::naked_asm;

macro_rules! csr {
    ($name:ident, $num:literal) => {
        pub struct $name;

        impl Csr for $name {
            const NUMBER: u16 = $num;
        }
    };
}

csr!(Cycle, 0xC00);
csr!(Time, 0xC01);
csr!(InstRet, 0xC02);
csr!(HpmCounter3, 0xC03);
csr!(HpmCounter4, 0xC04);
csr!(HpmCounter5, 0xC05);
csr!(HpmCounter6, 0xC06);
csr!(HpmCounter7, 0xC07);
csr!(HpmCounter8, 0xC08);
csr!(HpmCounter9, 0xC09);
csr!(HpmCounter10, 0xC0A);
csr!(HpmCounter11, 0xC0B);
csr!(HpmCounter12, 0xC0C);
csr!(HpmCounter13, 0xC0D);
csr!(HpmCounter14, 0xC0E);
csr!(HpmCounter15, 0xC0F);
csr!(HpmCounter16, 0xC10);
csr!(HpmCounter17, 0xC11);
csr!(HpmCounter18, 0xC12);
csr!(HpmCounter19, 0xC13);
csr!(HpmCounter20, 0xC14);
csr!(HpmCounter21, 0xC15);
csr!(HpmCounter22, 0xC16);
csr!(HpmCounter23, 0xC17);
csr!(HpmCounter24, 0xC18);
csr!(HpmCounter25, 0xC19);
csr!(HpmCounter26, 0xC1A);
csr!(HpmCounter27, 0xC1B);
csr!(HpmCounter28, 0xC1C);
csr!(HpmCounter29, 0xC1D);
csr!(HpmCounter30, 0xC1E);
csr!(HpmCounter31, 0xC1F);

csr!(Sstatus, 0x100);
csr!(Sie, 0x104);
csr!(Stvec, 0x105);
csr!(Scountern, 0x106);

csr!(SenvCfg, 0x10A);

csr!(Scountinhibit, 0x120);

csr!(Sscratch, 0x140);

csr!(Sepc, 0x141);
csr!(Scause, 0x142);
csr!(Stval, 0x143);
csr!(Sip, 0x144);
csr!(Scountovf, 0xDA0);

csr!(Satp, 0x180);

csr!(Scontext, 0x5A8);

csr!(Sstateen0, 0x10C);
csr!(Sstateen1, 0x10D);
csr!(Sstateen2, 0x10E);
csr!(Sstateen3, 0x10F);

pub trait Csr {
    const NUMBER: u16;

    #[naked]
    extern "C" fn swap(val: u64) -> u64 {
        unsafe {
            naked_asm!(
                "csrrw a0, {x}, a0",
                "ret",
                x = const Self::NUMBER
            )
        }
    }

    #[naked]
    extern "C" fn read() -> u64 {
        unsafe {
            naked_asm!(
                "csrrs a0, {x}, x0",
                "ret",
                x = const Self::NUMBER
            )
        }
    }

    #[naked]
    extern "C" fn write(val: u64) {
        unsafe {
            naked_asm!(
                "csrrw x0, {x}, a0",
                "ret",
                x = const Self::NUMBER
            )
        }
    }

    #[naked]
    extern "C" fn set_read(val: u64) -> u64 {
        unsafe {
            naked_asm!(
                "csrrs a0, {x}, a0",
                "ret",
                x = const Self::NUMBER
            )
        }
    }

    #[naked]
    extern "C" fn clear_read(val: u64) -> u64 {
        unsafe {
            naked_asm!(
                "csrrv a0, {x}, a0",
                "ret",
                x = const Self::NUMBER
            )
        }
    }
}
