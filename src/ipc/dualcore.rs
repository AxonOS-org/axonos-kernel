//! Dual-Core Contract Implementation
//!
//! Partition model:
//! - M4F (hard real-time): signal pipeline, consent state machine, stimulation interlock
//! - A53 (soft real-time): session management, BLE/Wi-Fi egress, WebAssembly sandbox
//! - Shared SRAM: 64-slot SPSC ring buffer (64 bytes/slot, 4096 bytes total)

use super::{DcClause, IpcLatency, ContractViolation};
use crate::ringbuf::SpscRingBuffer;
use crate::config;

/// Dual-core contract state machine
pub struct DualCoreContract {
    /// Contract clauses and their status
    clauses: [ClauseStatus; 6],
    /// Shared SPSC ring buffer
    shared_buffer: SpscRingBuffer<IntentPacket>,
    /// M4F heartbeat counter
    heartbeat_count: u32,
    /// Last heartbeat timestamp [µs]
    last_heartbeat: u64,
    /// DC5 safe-idle state
    safe_idle_active: bool,
    /// Violation log
    violations: heapless::Vec<ContractViolation, 16>,
}

/// Intent packet for IPC
#[derive(Debug, Clone, Copy)]
pub struct IntentPacket {
    /// Intent class
    pub class: u8,
    /// Confidence [0-255]
    pub confidence: u8,
    /// HMAC tag (truncated)
    pub hmac_tag: [u8; 4],
    /// Epoch index
    pub epoch: u64,
    /// Timestamp [µs]
    pub timestamp: u64,
}

/// Clause status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClauseStatus {
    /// Clause satisfied
    Satisfied,
    /// Clause under monitoring
    Monitoring,
    /// Clause violated
    Violated,
    /// Clause pending validation
    Pending,
}

impl DualCoreContract {
    /// Create new contract
    pub fn new() -> Self {
        Self {
            clauses: [ClauseStatus::Pending; 6],
            shared_buffer: SpscRingBuffer::new(),
            heartbeat_count: 0,
            last_heartbeat: 0,
            safe_idle_active: false,
            violations: heapless::Vec::new(),
        }
    }

    /// M4F side: send intent packet to A53
    pub fn send_intent(&self, packet: IntentPacket) -> Result<(), crate::ringbuf::SpscError> {
        self.shared_buffer.try_push(packet)
    }

    /// A53 side: receive intent packet from M4F
    pub fn receive_intent(&self) -> Result<IntentPacket, crate::ringbuf::SpscError> {
        self.shared_buffer.try_pop()
    }

    /// M4F side: send heartbeat
    ///
    /// DC5: If ≥3 consecutive heartbeats missed, A53 enters safe-idle.
    pub fn send_heartbeat(&mut self, timestamp: u64) {
        self.heartbeat_count += 1;
        self.last_heartbeat = timestamp;
        self.safe_idle_active = false;
        self.clauses[4] = ClauseStatus::Satisfied; // DC5
    }

    /// A53 side: check heartbeat
    ///
    /// Returns true if heartbeat is valid (within timeout).
    /// Returns false if timeout exceeded — must enter safe-idle.
    pub fn check_heartbeat(&mut self, now: u64) -> bool {
        let elapsed_ms = (now - self.last_heartbeat) / 1000;

        if elapsed_ms > config::SAFE_IDLE_TIMEOUT_MS as u64 {
            // Heartbeat lost — activate safe-idle
            self.safe_idle_active = true;
            self.clauses[4] = ClauseStatus::Violated;

            // Log violation
            let _ = self.violations.push(ContractViolation {
                clause: DcClause::Dc5,
                timestamp: now,
                observed: Some(elapsed_ms as f32),
                expected: Some(config::SAFE_IDLE_TIMEOUT_MS as f32),
            });

            false
        } else {
            true
        }
    }

    /// Check if safe-idle is active
    pub fn is_safe_idle(&self) -> bool {
        self.safe_idle_active
    }

    /// Get clause status
    pub fn clause_status(&self, clause: DcClause) -> ClauseStatus {
        match clause {
            DcClause::Dc1 => self.clauses[0],
            DcClause::Dc2 => self.clauses[1],
            DcClause::Dc3 => self.clauses[2],
            DcClause::Dc4 => self.clauses[3],
            DcClause::Dc5 => self.clauses[4],
            DcClause::Dc6 => self.clauses[5],
        }
    }

    /// Get all violations
    pub fn violations(&self) -> &[ContractViolation] {
        &self.violations
    }

    /// Reset contract state
    pub fn reset(&mut self) {
        self.heartbeat_count = 0;
        self.last_heartbeat = 0;
        self.safe_idle_active = false;
        self.violations.clear();
    }
}
