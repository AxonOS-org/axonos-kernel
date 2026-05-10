/* AxonOS Linker Script */

INCLUDE memory.x

ENTRY(Reset_Handler)

SECTIONS
{
  .text :
  {
    KEEP(*(.vector_table))
    *(.text .text.*)
    *(.rodata .rodata.*)
  } > FLASH

  .data : AT(ADDR(.text) + SIZEOF(.text))
  {
    _sdata = .;
    *(.data .data.*)
    _edata = .;
  } > RAM

  .bss :
  {
    _sbss = .;
    *(.bss .bss.*)
    _ebss = .;
  } > RAM

  /* FIR coefficients in CCM for fast access */
  .ccm :
  {
    *(.ccm .ccm.*)
  } > CCM
}
