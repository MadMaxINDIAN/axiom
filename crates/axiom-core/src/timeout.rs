// Timeout budget enforcement is integrated into evaluator::evaluate().
// This module provides the public constants and a small helper for
// other consumers (e.g. server batch evaluation).

use std::time::Instant;

/// Default evaluation timeout: 50 ms (matches §6.3 example).
pub const DEFAULT_TIMEOUT_MS: u64 = 50;

/// A simple deadline wrapper.
pub struct Deadline {
    start:      Instant,
    budget_us:  u64,
}

impl Deadline {
    pub fn new(budget_ms: u64) -> Self {
        Deadline { start: Instant::now(), budget_us: budget_ms * 1_000 }
    }

    pub fn elapsed_us(&self) -> u64 {
        self.start.elapsed().as_micros() as u64
    }

    pub fn is_exceeded(&self) -> bool {
        self.elapsed_us() >= self.budget_us
    }

    pub fn remaining_ms(&self) -> Option<u64> {
        let elapsed_ms = self.start.elapsed().as_millis() as u64;
        let budget_ms  = self.budget_us / 1_000;
        budget_ms.checked_sub(elapsed_ms)
    }
}
