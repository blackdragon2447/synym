OUTPUT_ARCH( "riscv" )

ENTRY( _start )

MEMORY
{
  kernel_space   (wxa) : ORIGIN = 0x80500000, LENGTH = 128M
}

PHDRS
{
  text PT_LOAD;
  data PT_LOAD;
  bss PT_LOAD;
}

SECTIONS
{
  .text : {
    PROVIDE(_text_start = .);
    *(.text.init) *(.text .text.*)
    PROVIDE(_text_end = .);
  } >kernel_space AT>kernel_space :text

  . = ALIGN(8);
  PROVIDE(__global_pointer$ = .);

  .rodata : {
    PROVIDE(_rodata_start = .);
    *(.rodata .rodata.*)
    PROVIDE(_rodata_end = .);
  } >kernel_space AT>kernel_space :text

  .data : {
    . = ALIGN(4096);
    PROVIDE(_data_start = .);
    *(.sdata .sdata.*) *(.data .data.*)
    PROVIDE(_data_end = .);
  } >kernel_space AT>kernel_space :data

  .bss :{
    PROVIDE(_bss_start = .);
    *(.sbss .sbss.*) *(.bss .bss.*)
    PROVIDE(_bss_end = .);
  } >kernel_space AT>kernel_space :bss

  PROVIDE(_memory_start = ORIGIN(kernel_space));
  PROVIDE(_stack_start = _bss_end);
  PROVIDE(_stack_end = _stack_start + 16K);
  PROVIDE(_heap_start = _stack_end);
  PROVIDE(_heap_size = _memory_end - _heap_start);

}
