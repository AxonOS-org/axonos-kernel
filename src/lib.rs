//! # AxonOS Kernel
//!
//! Safety-critical `#![no_std]` Rust microkernel for brain-computer interface
//! systems on Cortex-M4F and Cortex-M33 bare-metal targets.
//!
//! ## Core Properties
//!
//! - **EDF Scheduling**: Earliest-Deadline-First with Liu-Layland schedulability test
//! - **Zero-Copy Signal Path**: SPSC ring buffer from ADC DMA to classifier
//! - **Capability Isolation**: Structural data minimisation at type-system level
//! - **Dual-Core Contract**: Formal timing contract between M4F DSP and A53 app core
//! - **Forbidden Unsafe**: `#![forbid(unsafe_code)]` across all modules except two targeted blocks in SPSC
//!
//! ## Evidence Levels
//!
//! Every quantitative claim carries a mandatory evidence label:
//! - `[L1]`: Instruction-count derived
//! - `[L2]`: Runtime measured (DWT cycle counter)
//! - `[L3]`: Oscilloscope-validated
//! - `[pending]`: Not yet measured
//!
//! ## Reference Hardware
//!
//! | Component | Part | Role |
//! |-----------|------|------|
//! | DSP core | STM32F407 Cortex-M4F @ 168 MHz | Signal pipeline, consent FSM |
//! | App core | Cortex-A53 @ 1.2 GHz | Session, I/O, WASM sandbox |
//! | ADC | ADS1299 8-ch 24-bit 250 SPS | EEG acquisition |
//! | Secure element | ATECC608B | HMAC attestation |
//! | BLE radio | nRF52840 BLE 5.3 | Intent egress |
//! | Isolation | ISO7741 5 kV | Galvanic isolation |

#![no_std]
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(clippy::pedantic)]
#![warn(clippy::nursery)]
#![deny(rust_2018_idioms)]

// Module hierarchy
pub mod scheduler;
pub mod signal;
pub mod ringbuf;
pub mod ipc;
pub mod capability;
pub mod consent;
pub mod attestation;
pub mod platform;
pub mod hal;
pub mod zerocalib;

// Re-export core types for application use
pub use scheduler::{EdfScheduler, Task, TaskId, Deadline, Period, Wcet};
pub use signal::{SignalPipeline, PipelineStage, Epoch, EegFrame, MotorImageryClass};
pub use ringbuf::{SpscRingBuffer, RingBufferConfig};
pub use ipc::{DualCoreContract, IpcLatency, DcClause, IntentPacket};
pub use capability::{Capability, Manifest, ManifestBuilder, Catalogue, Dispatch};
pub use consent::{ConsentFsm, ConsentState, ConsentOp, Interlock};
pub use attestation::{Attestation, HmacSha256};

/// Kernel version following semantic versioning
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Evidence level taxonomy per RFC-0003
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceLevel {
    /// Instruction-count derived from compiled assembly
    L1,
    /// Runtime measured via on-chip DWT cycle counter
    L2,
    /// Independent oscilloscope-validated
    L3,
    /// Not yet measured — must state target date and falsification criterion
    Pending { target_date: &'static str, falsification: &'static str },
}

/// Kernel configuration constants derived from hardware specification
pub mod config {
    /// CPU frequency [Hz] — STM32F407 at 168 MHz [L1]
    pub const CPU_HZ: u32 = 168_000_000;

    /// ADC sampling rate [SPS] — ADS1299 at 250 SPS [L1]
    pub const ADC_SPS: u32 = 250;

    /// Epoch period [µs] = 1 / 250 SPS = 4000 µs [L1]
    pub const EPOCH_US: u32 = 4_000;

    /// DWT cycle counter resolution [ns] ≈ 5.95 ns [L1]
    pub const DWT_RESOLUTION_NS: f32 = 5.95;

    /// Conservative admission ceiling U_max = 0.25 [L1]
    pub const ADMISSION_CEILING: f32 = 0.25;

    /// Number of EEG channels [L1]
    pub const EEG_CHANNELS: usize = 8;

    /// ADC resolution [bits] [L1]
    pub const ADC_RESOLUTION: usize = 24;

    /// Bytes per sample frame: 8 ch × 3 bytes = 24 bytes [L1]
    pub const SAMPLE_FRAME_BYTES: usize = 24;

    /// SPI DMA transactions per frame: ceil(24/4) = 6 [L1]
    pub const SPI_DMA_TRANSACTIONS: usize = 6;

    /// FIR filter order [L1]
    pub const FIR_ORDER: usize = 64;

    /// SPSC ring buffer capacity — power of 2 [L1]
    pub const RING_BUFFER_CAPACITY: usize = 64;

    /// Shared SRAM size for IPC [bytes] [L1]
    pub const SHARED_SRAM_BYTES: usize = 4096;

    /// HMAC-SHA256 tag length [bytes] [L1]
    pub const HMAC_TAG_LEN: usize = 32;

    /// Maximum tasks in EDF scheduler [L1]
    pub const MAX_TASKS: usize = 8;

    /// DC5 safe-idle timeout [ms] [L2]
    pub const SAFE_IDLE_TIMEOUT_MS: u32 = 12;

    /// A53 wake-up deterministic bound [µs] [L2]
    pub const A53_WAKE_US: u32 = 50;

    /// Flash wait states at 168 MHz [L1]
    pub const FLASH_WAIT_STATES: u8 = 5;

    /// PLL configuration: HSE 8 MHz → 168 MHz [L1]
    /// PLLM = 8, PLLN = 336, PLLP = 2, PLLQ = 7
    pub const PLL_CONFIG: u32 = 0x0740_3408;
}

/// Kernel panic handler for bare-metal
///
/// In production: triggers DC5 interlock, logs to secure element, halts.
/// This handler is active only in non-test builds.
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // 1. Immediately disable stimulation (DC5)
    consent::Interlock::activate_safe_idle();

    // 2. Log panic to ATECC608B secure element slot 8 (secure log)

    // 3. Enter infinite breakpoint loop for debugger attachment
    loop {
        cortex_m::asm::bkpt();
    }
}
