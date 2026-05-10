//! Memory Barriers — Production Implementation
//!
//! ARMv7-M DMB/DSB/ISB instructions for cache coherency and ordering.
//! Required for DMA, multi-core, and memory-mapped peripheral access.

/// Memory barrier operations
pub struct MemoryBarrier;

impl MemoryBarrier {
    /// Data Memory Barrier
    ///
    /// Ensures that all explicit memory accesses before the DMB are completed
    /// before any explicit memory accesses after the DMB.
    /// Required when switching between DMA and CPU access to shared memory.
    #[inline(always)]
    pub fn dmb() {
        cortex_m::asm::dmb();
    }

    /// Data Synchronization Barrier
    ///
    /// Acts as DMB + flushes pipeline and ensures completion of all instructions.
    /// Required after changing MPU regions or cache settings.
    #[inline(always)]
    pub fn dsb() {
        cortex_m::asm::dsb();
    }

    /// Instruction Synchronization Barrier
    ///
    /// Flushes pipeline and ensures subsequent instructions are fetched with
    /// new context (e.g., after enabling/disabling interrupts).
    #[inline(always)]
    pub fn isb() {
        cortex_m::asm::isb();
    }

    /// Combined barrier for DMA buffer handoff
    ///
    /// Use after CPU writes to buffer before DMA starts,
    /// or after DMA completes before CPU reads.
    #[inline(always)]
    pub fn dma_buffer_handoff() {
        Self::dsb();
        Self::dmb();
    }

    /// Barrier after interrupt enable/disable
    #[inline(always)]
    pub fn irq_context_switch() {
        Self::dsb();
        Self::isb();
    }
}
