//! Platform Abstraction Layer
//!
//! Hardware-specific implementations for:
//! - STM32F407 (Cortex-M4F @ 168 MHz)
//! - STM32H573 (Cortex-M33 @ 250 MHz, TrustZone)

pub mod cortex_m4f;
pub mod cortex_m33;
pub mod dwt;
pub mod gpio;
pub mod dma;
pub mod spi;
pub mod adc;

pub use dwt::Dwt;
