//! Admission Controller
//!
//! Proposition 5.4: Conservative admission ceiling U_max = 0.25.
//! Binding utilisation estimate U^L2 = 0.2181 < U_max = 0.25.
//! Headroom: 0.0319 (3.2 percentage points).

use super::{Task, Wcet};
use crate::config;

/// Result of admission test
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionResult {
    /// Task admitted
    Admitted { utilisation: f32, headroom: f32 },
    /// Task rejected
    Rejected { reason: AdmissionReason },
}

/// Rejection reason
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionReason {
    /// Would exceed U_max ceiling
    CeilingExceeded,
    /// WCET exceeds period (C_i > T_i)
    WcetExceedsPeriod,
    /// Period too short for pipeline
    PeriodBelowMinimum,
    /// Task ID already in use
    DuplicateId,
}

/// Admission controller with conservative ceiling
pub struct AdmissionController {
    /// Current total utilisation
    current_util: f32,
    /// Maximum allowed utilisation
    max_util: f32,
}

impl AdmissionController {
    /// Create admission controller with default ceiling
    pub const fn new() -> Self {
        Self {
            current_util: 0.0,
            max_util: config::ADMISSION_CEILING,
        }
    }

    /// Create with custom ceiling (for testing)
    pub const fn with_ceiling(ceiling: f32) -> Self {
        Self {
            current_util: 0.0,
            max_util: ceiling,
        }
    }

    /// Test if task can be admitted
    ///
    /// Checks:
    /// 1. C_i ≤ T_i (WCET does not exceed period)
    /// 2. U + C_i/T_i ≤ U_max (ceiling not exceeded)
    /// 3. T_i ≥ EPOCH_US (period at least one epoch)
    pub fn test(&self, task: &Task) -> AdmissionResult {
        // Check 1: WCET ≤ Period
        if task.wcet.0 > task.period.0 {
            return AdmissionResult::Rejected {
                reason: AdmissionReason::WcetExceedsPeriod,
            };
        }

        // Check 2: Period minimum
        if task.period.0 < config::EPOCH_US {
            return AdmissionResult::Rejected {
                reason: AdmissionReason::PeriodBelowMinimum,
            };
        }

        // Check 3: Utilisation ceiling
        let new_util = self.current_util + task.utilisation;
        if new_util > self.max_util {
            return AdmissionResult::Rejected {
                reason: AdmissionReason::CeilingExceeded,
            };
        }

        AdmissionResult::Admitted {
            utilisation: new_util,
            headroom: self.max_util - new_util,
        }
    }

    /// Admit task (mutates controller state)
    pub fn admit(&mut self, task: &Task) -> AdmissionResult {
        let result = self.test(task);
        if let AdmissionResult::Admitted { utilisation, .. } = result {
            self.current_util = utilisation;
        }
        result
    }

    /// Current headroom
    pub fn headroom(&self) -> f32 {
        self.max_util - self.current_util
    }

    /// Reset controller
    pub fn reset(&mut self) {
        self.current_util = 0.0;
    }
}

/// Pre-defined AxonOS task set (Table 4)
pub fn axonos_task_set() -> [Task; 5] {
    [
        // τ1: Signal pipeline (dominant task)
        Task::new(1, 818, 4000, "signal_pipeline"), // C_1^L2 = 818 µs [L2]
        // τ2: Consent state machine
        Task::new(2, 12, 4000, "consent_fsm"),
        // τ3: HMAC attestation
        Task::new(3, 18, 4000, "hmac_attestation"),
        // τ4: BLE intent egress
        Task::new(4, 24, 4000, "ble_egress"),
        // τ5: Background diagnostics
        Task::new(5, 100, 1_000_000, "background_diagnostics"),
    ]
}

/// Compute total utilisation of task set
pub fn total_utilisation(tasks: &[Task]) -> f32 {
    tasks.iter().map(|t| t.utilisation).sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_axonos_task_set_utilisation() {
        let tasks = axonos_task_set();
        let u = total_utilisation(&tasks);
        // U^L2 = 0.2181 [L2]
        assert!((u - 0.2181).abs() < 0.001);
    }

    #[test]
    fn test_admission_ceiling() {
        let mut ctrl = AdmissionController::new();
        let tasks = axonos_task_set();

        for task in &tasks {
            assert!(matches!(ctrl.admit(task), AdmissionResult::Admitted { .. }));
        }

        // Should have ~0.0319 headroom
        assert!(ctrl.headroom() > 0.03);
        assert!(ctrl.headroom() < 0.04);
    }

    #[test]
    fn test_reject_over_ceiling() {
        let mut ctrl = AdmissionController::with_ceiling(0.1);
        let task = Task::new(99, 500, 4000, "huge_task");
        assert!(matches!(
            ctrl.admit(&task),
            AdmissionResult::Rejected { reason: AdmissionReason::CeilingExceeded }
        ));
    }
}
