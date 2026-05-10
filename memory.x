/* AxonOS Memory Layout — STM32F407 */

MEMORY
{
  /* Flash: 1 MB */
  FLASH (rx)  : ORIGIN = 0x08000000, LENGTH = 1024K

  /* SRAM: 192 KB (128 KB contiguous + 64 KB CCM) */
  RAM (rwx)   : ORIGIN = 0x20000000, LENGTH = 128K
  CCM (rwx)   : ORIGIN = 0x10000000, LENGTH = 64K
}

/* Stack top: end of RAM */
_stack_top = ORIGIN(RAM) + LENGTH(RAM);
