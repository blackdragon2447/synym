.section .text.init
.global _start
_start:
.option push
.option norelax
    la      gp, __global_pointer$
    la      t3, _bss_start
    la      t4, _bss_end
.option pop
    # Clear BSS
1:
    bge     t3, t4, 1f
    sd      zero, 0(t3)
    addi    t3, t3, 8
    j       1b
1:
    la      sp, _stack_end
    li      t0, (1 << 8) | (1 << 5) | (1 << 13)
    csrw    sstatus, t0
    la      t1, kernel_main
    csrw    sie, zero
    csrw    sepc, t1
    sret
4:
    wfi
    j       4b
