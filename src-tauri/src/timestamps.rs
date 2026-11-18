//! Persisted library timestamps use Unix milliseconds (not seconds).
use std::time::{SystemTime, UNIX_EPOCH};

pub fn unix_millis(time: SystemTime) -> Option<i64> {
    let elapsed = time.duration_since(UNIX_EPOCH).ok()?;
    Some(elapsed.as_millis().min(i64::MAX as u128) as i64)
}
pub fn unix_millis_or_zero(time: SystemTime) -> i64 { unix_millis(time).unwrap_or_default() }
pub fn now() -> i64 { unix_millis_or_zero(SystemTime::now()) }

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn millisecond_precision_and_pre_epoch_policy() {
        assert_eq!(unix_millis(UNIX_EPOCH + Duration::from_micros(1_234_567)), Some(1234));
        assert_eq!(unix_millis(UNIX_EPOCH), Some(0));
        assert_eq!(unix_millis(UNIX_EPOCH - Duration::from_millis(1)), None);
        assert_eq!(unix_millis_or_zero(UNIX_EPOCH - Duration::from_millis(1)), 0);
    }
}
