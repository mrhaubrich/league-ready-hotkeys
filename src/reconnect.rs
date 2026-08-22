use std::time::Duration;

#[derive(Debug, Clone, Copy)]
pub struct ReconnectPolicy { pub base: Duration, pub cap: Duration }

impl Default for ReconnectPolicy {
    fn default() -> Self { Self { base: Duration::from_millis(250), cap: Duration::from_secs(5) } }
}

impl ReconnectPolicy {
    pub fn delay(self, attempt: u32) -> Duration {
        let factor = 2u32.saturating_pow(attempt.min(6));
        (self.base * factor).min(self.cap)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backoff_is_capped() {
        let policy = ReconnectPolicy::default();
        assert_eq!(policy.delay(0), Duration::from_millis(250));
        assert_eq!(policy.delay(4), Duration::from_secs(4));
        assert_eq!(policy.delay(20), Duration::from_secs(5));
    }
}
