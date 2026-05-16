/* Linker script for STM32F407VG (1 MB flash, 192 KB SRAM including CCM).
 * For other STM32F407 variants, adjust LENGTH accordingly.
 *
 * Memory map (RM0090):
 *   FLASH   : 0x08000000, 1024 KB
 *   SRAM1   : 0x20000000,  112 KB
 *   SRAM2   : 0x2001C000,   16 KB
 *   CCMRAM  : 0x10000000,   64 KB (Cortex-M4 Core-Coupled Memory; not DMA-accessible)
 *
 * We combine SRAM1 and SRAM2 into one contiguous 128 KB RAM region.
 * CCMRAM is declared separately for code that does not require DMA
 * (e.g., scheduler state).
 */
MEMORY
{
    FLASH  : ORIGIN = 0x08000000, LENGTH = 1024K
    RAM    : ORIGIN = 0x20000000, LENGTH = 128K
    CCMRAM : ORIGIN = 0x10000000, LENGTH = 64K
}
