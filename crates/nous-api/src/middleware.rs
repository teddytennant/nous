use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tracing::info;

pub async fn request_logger(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    info!(
        %method,
        %uri,
        %status,
        duration_ms = duration.as_millis(),
        "request completed"
    );

    response
}

pub struct RateLimiter {
    requests: Vec<Instant>,
    max_rpm: u32,
}

impl RateLimiter {
    pub fn new(max_rpm: u32) -> Arc<Mutex<Self>> {
        Arc::new(Mutex::new(Self {
            requests: Vec::new(),
            max_rpm,
        }))
    }

    pub fn check(&mut self) -> bool {
        let now = Instant::now();
        let one_minute_ago = now - std::time::Duration::from_secs(60);

        self.requests.retain(|&t| t > one_minute_ago);

        if self.requests.len() < self.max_rpm as usize {
            self.requests.push(now);
            true
        } else {
            false
        }
    }

    pub fn remaining(&self) -> u32 {
        let now = Instant::now();
        let one_minute_ago = now - std::time::Duration::from_secs(60);
        let recent = self
            .requests
            .iter()
            .filter(|&&t| t > one_minute_ago)
            .count();
        self.max_rpm.saturating_sub(recent as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rate_limiter_allows_under_limit() {
        let limiter = RateLimiter::new(10);
        let mut guard = limiter.lock().await;
        assert!(guard.check());
        assert_eq!(guard.remaining(), 9);
    }

    #[tokio::test]
    async fn rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(3);
        let mut guard = limiter.lock().await;
        assert!(guard.check());
        assert!(guard.check());
        assert!(guard.check());
        assert!(!guard.check());
    }

    #[tokio::test]
    async fn rate_limiter_remaining() {
        let limiter = RateLimiter::new(100);
        let mut guard = limiter.lock().await;
        assert_eq!(guard.remaining(), 100);
        guard.check();
        guard.check();
        assert_eq!(guard.remaining(), 98);
    }
}
