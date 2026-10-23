use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Default)]
struct Bucket {
    attempts: Vec<Instant>,
    blocked_until: Option<Instant>,
}

/// Login protection with two dimensions:
///
/// * `u:<username>|<ip>` locks a single identity after repeated failures.
/// * `ip:<ip>` catches spraying across many usernames from one address.
///
/// Once the threshold is crossed the key is locked for `lockout` seconds, so
/// waiting out the sliding window is not enough to keep guessing.
pub struct LoginLimiter {
    max_attempts: u32,
    ip_max_attempts: u32,
    window: Duration,
    lockout: Duration,
    buckets: Mutex<HashMap<String, Bucket>>,
}

impl LoginLimiter {
    pub fn new(
        max_attempts: u32,
        ip_max_attempts: u32,
        window_secs: u64,
        lockout_secs: u64,
    ) -> Self {
        Self {
            max_attempts: max_attempts.max(1),
            ip_max_attempts: ip_max_attempts.max(1),
            window: Duration::from_secs(window_secs.max(10)),
            lockout: Duration::from_secs(lockout_secs.max(10)),
            buckets: Mutex::new(HashMap::new()),
        }
    }

    fn limit_for(&self, key: &str) -> u32 {
        if key.starts_with("ip:") {
            self.ip_max_attempts
        } else {
            self.max_attempts
        }
    }

    fn housekeeping(&self, map: &mut HashMap<String, Bucket>, now: Instant) {
        let cutoff = now.checked_sub(self.window).unwrap_or(now);
        map.retain(|_, bucket| {
            bucket.attempts.retain(|instant| *instant > cutoff);
            let still_blocked = bucket
                .blocked_until
                .map(|until| until > now)
                .unwrap_or(false);
            still_blocked || !bucket.attempts.is_empty()
        });
    }

    /// Seconds until at least one of the keys is unlocked.
    pub fn retry_after(&self, keys: &[&str]) -> Option<u64> {
        let Ok(mut map) = self.buckets.lock() else {
            return Some(60);
        };
        let now = Instant::now();
        self.housekeeping(&mut map, now);
        let mut wait: Option<u64> = None;
        for key in keys {
            if let Some(until) = map.get(*key).and_then(|bucket| bucket.blocked_until) {
                if until > now {
                    let seconds = (until - now).as_secs().max(1);
                    wait = Some(wait.map(|current| current.max(seconds)).unwrap_or(seconds));
                }
            }
        }
        wait
    }

    pub fn record_failure(&self, keys: &[&str]) {
        let Ok(mut map) = self.buckets.lock() else {
            return;
        };
        let now = Instant::now();
        self.housekeeping(&mut map, now);
        for key in keys {
            let bucket = map.entry((*key).to_string()).or_default();
            bucket.attempts.push(now);
            let limit = self.limit_for(key) as usize;
            if bucket.attempts.len() >= limit {
                bucket.attempts.clear();
                bucket.blocked_until = Some(now + self.lockout);
            }
        }
    }

    pub fn reset(&self, key: &str) {
        if let Ok(mut map) = self.buckets.lock() {
            map.remove(key);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locks_identity_after_threshold() {
        let limiter = LoginLimiter::new(3, 3, 60, 60);
        let keys = ["u:admin|1.2.3.4", "ip:1.2.3.4"];
        assert!(limiter.retry_after(&keys).is_none());
        limiter.record_failure(&keys);
        limiter.record_failure(&keys);
        assert!(limiter.retry_after(&keys).is_none());
        limiter.record_failure(&keys);
        assert!(limiter.retry_after(&keys).is_some());
        limiter.reset("u:admin|1.2.3.4");
        assert!(limiter.retry_after(&keys).is_some(), "IP lock remains");
    }

    #[test]
    fn ip_threshold_is_independent() {
        let limiter = LoginLimiter::new(50, 2, 60, 60);
        let ip = "ip:9.9.9.9";
        limiter.record_failure(&[ip]);
        limiter.record_failure(&[ip]);
        assert!(limiter.retry_after(&[ip]).is_some());
        assert!(limiter.retry_after(&["u:other|9.9.9.9"]).is_none());
    }
}
