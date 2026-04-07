/// Token-bucket rate limiter per API key (§6.4).
///
/// Default: 1,000 req/s per key.  Batch requests count as N evaluations.
/// Returns HTTP 429 with `Retry-After` header when exceeded.
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;

#[derive(Debug)]
struct Bucket {
    tokens:       f64,
    last_refill:  Instant,
    rate_per_sec: f64,
    capacity:     f64,
}

impl Bucket {
    fn new(rate_per_sec: f64) -> Self {
        Bucket {
            tokens:      rate_per_sec,
            last_refill: Instant::now(),
            rate_per_sec,
            capacity: rate_per_sec,
        }
    }

    /// Try to consume `n` tokens. Returns `Ok(())` if allowed,
    /// `Err(retry_after_secs)` if rate-limited.
    fn consume(&mut self, n: f64) -> Result<(), f64> {
        let now     = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        self.tokens = (self.tokens + elapsed * self.rate_per_sec).min(self.capacity);
        self.last_refill = now;

        if self.tokens >= n {
            self.tokens -= n;
            Ok(())
        } else {
            // Seconds until enough tokens are available
            let retry = (n - self.tokens) / self.rate_per_sec;
            Err(retry.ceil())
        }
    }
}

pub struct RateLimiter {
    buckets:      Mutex<HashMap<String, Bucket>>,
    rate_per_sec: f64,
}

impl RateLimiter {
    pub fn new(rate_per_sec: u32) -> Self {
        RateLimiter {
            buckets:      Mutex::new(HashMap::new()),
            rate_per_sec: rate_per_sec as f64,
        }
    }

    /// Try to consume `n` tokens for `key_id`.
    /// Returns `Ok(())` or `Err(retry_after_secs)`.
    pub fn check(&self, key_id: &str, n: u32) -> Result<(), u64> {
        let mut buckets = self.buckets.lock().unwrap();
        let rate        = self.rate_per_sec;
        let bucket      = buckets
            .entry(key_id.to_string())
            .or_insert_with(|| Bucket::new(rate));
        bucket.consume(n as f64).map_err(|r| r as u64)
    }
}
