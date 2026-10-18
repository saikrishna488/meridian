//! Bounded retry state, independent of GTK so crash loops are testable.
use std::time::{Duration, Instant};

#[derive(Default)]
pub struct Recovery {
    failures: u32,
    last_failure: Option<Instant>,
}
impl Recovery {
    pub fn failed(&mut self, now: Instant) -> Duration {
        if self.last_failure.is_none_or(|last| now.duration_since(last) > Duration::from_secs(60)) {
            self.failures = 0;
        }
        self.last_failure = Some(now);
        let delay = Duration::from_millis((250u64 << self.failures.min(7)).min(30_000));
        self.failures = self.failures.saturating_add(1);
        delay
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crash_loops_back_off_and_stable_surfaces_recover_quickly() {
        let mut state = Recovery::default();
        let now = Instant::now();
        assert_eq!(state.failed(now), Duration::from_millis(250));
        assert_eq!(state.failed(now), Duration::from_millis(500));
        for _ in 0..1000 {
            assert!(state.failed(now) <= Duration::from_secs(30));
        }
        assert_eq!(state.failed(now + Duration::from_secs(61)), Duration::from_millis(250));
    }
}
