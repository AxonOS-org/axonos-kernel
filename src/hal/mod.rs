//! Hardware Abstraction Layer
//!
//! Critical section management, memory barriers, IRQ control.

pub mod critical_section;
pub mod memory;
pub mod irq;

pub use critical_section::CriticalSection;
pub use memory::MemoryBarrier;
pub use irq::IrqController;
