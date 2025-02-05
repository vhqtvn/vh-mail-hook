use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::fmt::Display;

type RateLimiterMap = Arc<Mutex<HashMap<ResourceKey, Arc<Mutex<RateLimiter>>>>>;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ResourceKey(String);

impl From<String> for ResourceKey {
    fn from(value: String) -> Self {
        ResourceKey(value)
    }
}

impl From<&str> for ResourceKey {
    fn from(value: &str) -> Self {
        ResourceKey(value.to_string())
    }
}

impl<T1: Display, T2: Display> From<(T1, T2)> for ResourceKey {
    fn from((prefix, id): (T1, T2)) -> Self {
        ResourceKey(format!("{}:{}", prefix, id))
    }
}

#[derive(Debug, Clone)]
pub struct RateLimitRule {
    pub max_requests: u32,
    pub period: Duration,
}

impl RateLimitRule {
    pub fn new(max_requests: u32, period_seconds: u32) -> Self {
        Self {
            max_requests,
            period: Duration::from_secs(period_seconds as u64),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    pub rules: Vec<RateLimitRule>,
}

impl From<Vec<RateLimitRule>> for RateLimiterConfig {
    fn from(rules: Vec<RateLimitRule>) -> Self {
        RateLimiterConfig { rules }
    }
}

impl<F> From<F> for RateLimiterConfig 
where 
    F: FnOnce() -> Vec<RateLimitRule>
{
    fn from(f: F) -> Self {
        RateLimiterConfig {
            rules: f(),
        }
    }
}

#[derive(Clone, Debug)]
struct WindowCounter {
    count: u32,
    window_start: Instant,
}

impl WindowCounter {
    fn new() -> Self {
        Self {
            count: 0,
            window_start: Instant::now(),
        }
    }

    fn increment(&mut self, period: Duration) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.window_start);
        
        if elapsed >= period {
            // Start a new window
            self.count = 1;
            self.window_start = now;
            true
        } else {
            // Still in current window
            self.count = self.count.saturating_add(1);
            true
        }
    }

    fn is_within_limit(&self, max_requests: u32) -> bool {
        // Check if window has expired
        let now = Instant::now();
        let elapsed = now.duration_since(self.window_start);
        
        if elapsed >= Duration::from_secs(0) {
            self.count < max_requests
        } else {
            // If time went backwards, be conservative and deny
            false
        }
    }
}

#[derive(Debug)]
pub struct RateLimiter {
    rules: Vec<RateLimitRule>,
    windows: Vec<WindowCounter>,
}

impl RateLimiter {
    fn new<T: Into<RateLimiterConfig>>(config: T) -> Self {
        let config = config.into();
        let length = config.rules.len();
        Self {
            rules: config.rules,
            windows: vec![WindowCounter::new(); length],
        }
    }

    pub fn trigger(&mut self) -> bool {
        for (rule, window) in self.rules.iter().zip(self.windows.iter_mut()) {
            if !window.is_within_limit(rule.max_requests) {
                return false;
            }
            if !window.increment(rule.period) {
                return false;
            }
        }
        true
    }
}

static RATE_LIMITERS: Lazy<RateLimiterMap> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub fn get_or_create_rate_limiter<K, C>(key: K, config: C) -> Arc<Mutex<RateLimiter>>
where
    K: Into<ResourceKey>,
    C: Into<RateLimiterConfig>,
{
    let key = key.into();
    let mut limiters = RATE_LIMITERS.lock().unwrap();

    if let Some(limiter) = limiters.get(&key) {
        limiter.clone()
    } else {
        let limiter = Arc::new(Mutex::new(RateLimiter::new(config.into())));
        limiters.insert(key, limiter.clone());
        limiter
    }
}
