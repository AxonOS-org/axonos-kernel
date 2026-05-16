// SPDX-License-Identifier: Apache-2.0 OR MIT
// Copyright (c) 2026 Denis Yermakou <denis@axonos.org>
// Part of the AxonOS project — https://github.com/AxonOS-org

//! # axonos-firmware-stm32f407
//!
//! Bare-metal Cortex-M4F firmware for the AxonOS reference platform.
//!
//! This binary boots the `axonos-kernel-core` integration on a real
//! STM32F407 (or compatible Cortex-M4F at 168 MHz), wires the DWT cycle
//! counter as the [`MonotonicClock`], and runs the 4-millisecond BCI
//! signal tick.
//!
//! ## Build
//!
//! ```sh
//! rustup target add thumbv7em-none-eabihf
//! cargo build --release --target thumbv7em-none-eabihf
//! ```
//!
//! The resulting ELF binary is at
//! `target/thumbv7em-none-eabihf/release/axonos-firmware-stm32f407`.
//!
//! ## Flash and run
//!
//! On STM32F407 Discovery board:
//!
//! ```sh
//! cargo install probe-rs --features cli
//! probe-rs run --chip STM32F407VGTx target/thumbv7em-none-eabihf/release/axonos-firmware-stm32f407
//! ```
//!
//! On QEMU (development only):
//!
//! ```sh
//! qemu-system-arm -cpu cortex-m4 -machine netduino2 \
//!     -kernel target/thumbv7em-none-eabihf/release/axonos-firmware-stm32f407 \
//!     -semihosting-config enable=on,target=native -nographic
//! ```
//!
//! ## What this firmware does
//!
//! 1. Enables the DWT cycle counter at boot.
//! 2. Constructs the BCI task set (signal pipeline, consent FSM, HMAC,
//!    BLE egress) and admits it under Liu–Layland at `U_max = 0.25`.
//! 3. Validates the application manifest against the kernel catalogue.
//! 4. Enters a 4-millisecond tick loop that picks the next task by EDF,
//!    generates a synthetic Navigation observation, validates against the
//!    capability gate, and pushes the encoded record into the SPSC IPC ring.
//! 5. A separate consumer (in a real system, a BLE egress task) drains
//!    the ring.
//!
//! This is a **scaffolding firmware**: it demonstrates that the five
//! foundational crates and the kernel-core integration compile and run
//! on real Cortex-M4F hardware, with a real time source. It does not
//! yet include the signal-processing pipeline kernels (FIR, CSP, LDA,
//! Riemannian classifier); those are stubbed as constant-time WCET
//! placeholders.
//!
//! ## Verification status
//!
//! This binary has not been executed on hardware in the repository CI.
//! It is verified only at the build level: `cargo build --release
//! --target thumbv7em-none-eabihf` must succeed. Phase-1 measurement
//! (Q2 2026) will exercise this firmware on a GPIO-instrumented
//! reference fixture with falsification thresholds set in advance per
//! the published preprint.

#![no_std]
#![no_main]
#![forbid(unsafe_code)]

use cortex_m::peripheral::DWT;
use cortex_m_rt::entry;
use panic_halt as _;

use axonos_capability::{Capability, CapabilitySet, Manifest};
use axonos_intent::{Confidence, NavigationDirection};
use axonos_kernel_core::{new_ipc_channel, BciKernel, KernelConfig};
use axonos_scheduler::{Micros, Task, TaskId};
use axonos_time::{Instant, MonotonicClock};

// ───────────────────────────────────────────────────────────────────────────
// DWT-backed monotonic clock
// ───────────────────────────────────────────────────────────────────────────

/// CPU frequency of the STM32F407 reference platform, in megahertz.
///
/// The DWT cycle counter is a 32-bit counter that increments at the CPU
/// frequency; dividing by this constant converts cycles to microseconds.
/// At 168 MHz the counter wraps every approximately 25.6 seconds. The
/// implementation below uses a wrap-tracking extension to 64 bits via
/// a low-priority systick interrupt expected to fire at least every
/// 12 seconds (half the wrap period).
pub const CPU_FREQ_MHZ: u32 = 168;

/// A `MonotonicClock` backed by the Cortex-M DWT cycle counter.
///
/// This implementation is the reference pattern from the
/// `axonos-time` docs, with the wrap-tracking extension stubbed for
/// simplicity. A production deployment must implement the wrap-tracking
/// helper to extend the 32-bit DWT counter to 64 bits.
pub struct DwtClock;

impl DwtClock {
    /// Construct a DWT clock. The DWT cycle counter MUST be enabled by
    /// the caller before any call to [`Self::now`].
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Enable the DWT trace unit and cycle counter. Call once at boot.
    pub fn enable(cp: &mut cortex_m::Peripherals) {
        // The DWT enable API is safe in the cortex-m crate.
        cp.DCB.enable_trace();
        cp.DWT.enable_cycle_counter();
    }
}

impl Default for DwtClock {
    fn default() -> Self {
        Self::new()
    }
}

impl MonotonicClock for DwtClock {
    fn now(&self) -> Instant {
        // DWT::cycle_count returns u32, which wraps every ~25.6 s at
        // 168 MHz. A production implementation extends this to u64 via
        // wrap-tracking; here we return the raw cycle count converted to
        // microseconds for demonstration purposes.
        let cycles = DWT::cycle_count();
        Instant(u64::from(cycles) / u64::from(CPU_FREQ_MHZ))
    }
}

// ───────────────────────────────────────────────────────────────────────────
// Kernel construction
// ───────────────────────────────────────────────────────────────────────────

/// Build the AxonOS BCI kernel for the reference task set.
fn build_kernel() -> Result<BciKernel<DwtClock, 8, 64>, axonos_kernel_core::KernelInitError> {
    let mut config: KernelConfig<8, 64> = KernelConfig::new();

    // The reference BCI signal pipeline, with WCETs from the preprint.
    // These are nominal values; real WCETs will be measured per the
    // Phase-1 falsification protocol (Q2 2026, RFC-0003).
    config
        .add_task(Task::periodic(TaskId(1), Micros(642), Micros(4000)))
        .ok();
    config
        .add_task(Task::periodic(TaskId(2), Micros(12), Micros(4000)))
        .ok();
    config
        .add_task(Task::periodic(TaskId(3), Micros(18), Micros(4000)))
        .ok();
    config
        .add_task(Task::periodic(TaskId(4), Micros(24), Micros(4000)))
        .ok();
    config
        .add_task(Task::periodic(TaskId(5), Micros(100), Micros(1_000_000)))
        .ok();

    // Default application manifest: Navigation + SessionQuality.
    let manifest = Manifest::new(
        CapabilitySet::singleton(Capability::Navigation).with(Capability::SessionQuality),
    );

    BciKernel::new(config, manifest, DwtClock::new())
}

// ───────────────────────────────────────────────────────────────────────────
// Entry point
// ───────────────────────────────────────────────────────────────────────────

#[entry]
fn main() -> ! {
    // Acquire the Cortex-M peripherals and enable the DWT cycle counter.
    let mut cp = cortex_m::Peripherals::take().expect("Peripherals available at boot");
    DwtClock::enable(&mut cp);

    // Construct the kernel. Any admission failure halts the firmware in
    // a known state, which panic_halt converts to a controlled fault.
    let mut kernel = build_kernel().expect("BCI task set must admit at U_max = 0.25");

    // Statically allocated IPC channel. In a more complete firmware this
    // would live in a static and be initialised once via OnceCell.
    let ipc = new_ipc_channel::<64>();
    let (mut producer, _consumer) = ipc.split().expect("first split must succeed");

    // 4-millisecond tick loop.
    let mut sequence = 0u32;
    loop {
        // Block until the next 4-ms boundary. In a real firmware this
        // would be driven by an ADC DMA-complete interrupt or a SysTick;
        // here we spin on the DWT clock for clarity.
        let now = kernel.now();
        let next_deadline = now.add_micros(axonos_time::Micros(4_000));
        while kernel.now() < next_deadline {
            cortex_m::asm::nop();
        }

        // Produce one synthetic Navigation observation. In a real
        // firmware this would source the observation from the signal-
        // processing pipeline.
        let direction = match sequence % 5 {
            0 => NavigationDirection::Idle,
            1 => NavigationDirection::Left,
            2 => NavigationDirection::Right,
            3 => NavigationDirection::Up,
            _ => NavigationDirection::Down,
        };
        let confidence = Confidence::from_q0_16(0x8000); // ≈ 0.5
        let _ = kernel.produce_observation(&mut producer, direction, confidence);

        sequence = sequence.wrapping_add(1);
    }
}
