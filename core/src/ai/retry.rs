use reqwest::StatusCode;
use std::time::{Duration, SystemTime};

/// Policy parameters for retry decisions.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    /// Delay used for the first retry attempt.
    pub base_delay: Duration,
    /// Maximum backoff delay that will be used, regardless of exponentiation or hints.
    pub max_delay: Duration,
    /// Maximum number of retry attempts allowed.
    pub max_retries: u32,
}

impl RetryPolicy {
    /// Creates a new [`RetryPolicy`].
    pub const fn new(base_delay: Duration, max_delay: Duration, max_retries: u32) -> Self {
        Self {
            base_delay,
            max_delay,
            max_retries,
        }
    }
}

/// Extra delay information returned by the server.
#[derive(Debug, Clone, Copy)]
pub struct RetryHint {
    delay: Duration,
}

impl RetryHint {
    pub const fn new(delay: Duration) -> Self {
        Self { delay }
    }

    pub const fn delay(&self) -> Duration {
        self.delay
    }
}

/// Error classes that influence retry decisions.
#[derive(Debug, Clone, Copy)]
pub enum RetryError {
    /// HTTP error with status code and optional server provided hint.
    Http {
        status: StatusCode,
        retry_hint: Option<RetryHint>,
    },
    /// Network level failure without a status code.
    Network { retry_hint: Option<RetryHint> },
    /// Errors that should not be retried.
    Fatal,
}

impl RetryError {
    fn hint(&self) -> Option<RetryHint> {
        match self {
            RetryError::Http { retry_hint, .. } | RetryError::Network { retry_hint, .. } => {
                *retry_hint
            }
            RetryError::Fatal => None,
        }
    }
}

/// Decision made by the retry policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryDecision {
    pub should_retry: bool,
    pub delay_ms: u64,
    pub used_hint: bool,
}

impl RetryDecision {
    const fn no_retry() -> Self {
        Self {
            should_retry: false,
            delay_ms: 0,
            used_hint: false,
        }
    }

    fn retry_with(delay: Duration, used_hint: bool, max_delay: Duration) -> Self {
        let capped = if delay > max_delay { max_delay } else { delay };
        let millis = capped.as_millis();
        let delay_ms = if millis > u64::MAX as u128 {
            u64::MAX
        } else {
            millis as u64
        };
        Self {
            should_retry: true,
            delay_ms,
            used_hint,
        }
    }
}

/// Calculates the next retry decision based on the provided error, policy and number of previous attempts.
///
/// * `previous_attempts` counts the number of retries that have already been made.
pub fn evaluate_retry(
    error: RetryError,
    policy: RetryPolicy,
    previous_attempts: u32,
) -> RetryDecision {
    if previous_attempts >= policy.max_retries {
        return RetryDecision::no_retry();
    }

    match error {
        RetryError::Fatal => RetryDecision::no_retry(),
        RetryError::Http { status, .. } if !is_retryable_status(status) => {
            RetryDecision::no_retry()
        }
        RetryError::Http { .. } | RetryError::Network { .. } => {
            if let Some(hint) = error.hint() {
                return RetryDecision::retry_with(hint.delay(), true, policy.max_delay);
            }

            let delay =
                compute_exponential_backoff(policy.base_delay, policy.max_delay, previous_attempts);
            RetryDecision::retry_with(delay, false, policy.max_delay)
        }
    }
}

fn is_retryable_status(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS
        || status == StatusCode::REQUEST_TIMEOUT
        || status.is_server_error()
}

fn compute_exponential_backoff(
    base: Duration,
    max_delay: Duration,
    previous_attempts: u32,
) -> Duration {
    if base.is_zero() {
        return Duration::from_millis(0);
    }

    let base_ms = base.as_millis();
    let max_ms = max_delay.as_millis();

    let mut multiplier: u128 = 1;
    for _ in 0..previous_attempts {
        multiplier = multiplier.saturating_mul(2);
    }

    let delay_ms = base_ms.saturating_mul(multiplier);
    let capped_ms = delay_ms.min(max_ms);
    Duration::from_millis(capped_ms as u64)
}

/// Parses the value of an HTTP `Retry-After` header.
///
/// Returns `None` when parsing fails.
pub fn parse_retry_after(value: &str, now: SystemTime) -> Option<Duration> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(seconds) = trimmed.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }

    if let Ok(instant) = httpdate::parse_http_date(trimmed) {
        if let Ok(duration) = instant.duration_since(now) {
            return Some(duration);
        }
        return Some(Duration::from_secs(0));
    }

    None
}

/// Parses the `retryDelay` string returned from Gemini's `RetryInfo` object.
///
/// The value is typically a string such as `"3s"` or `"1.5s"`.
pub fn parse_gemini_retry_delay(value: &str) -> Option<Duration> {
    let trimmed = value.trim();
    let Some(stripped) = trimmed.strip_suffix('s') else {
        return None;
    };

    let seconds = stripped.parse::<f64>().ok()?;
    if !seconds.is_finite() || seconds.is_sign_negative() {
        return None;
    }

    Some(Duration::from_secs_f64(seconds))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    const POLICY: RetryPolicy = RetryPolicy {
        base_delay: Duration::from_secs(1),
        max_delay: Duration::from_secs(30),
        max_retries: 5,
    };

    #[test]
    fn uses_hint_delay_when_available() {
        let decision = evaluate_retry(
            RetryError::Http {
                status: StatusCode::TOO_MANY_REQUESTS,
                retry_hint: Some(RetryHint::new(Duration::from_secs(19))),
            },
            POLICY,
            0,
        );

        assert_eq!(decision.should_retry, true);
        assert_eq!(decision.used_hint, true);
        assert_eq!(decision.delay_ms, 19_000);
    }

    #[test]
    fn exponential_backoff_without_hint() {
        let first = evaluate_retry(
            RetryError::Http {
                status: StatusCode::TOO_MANY_REQUESTS,
                retry_hint: None,
            },
            POLICY,
            0,
        );
        let second = evaluate_retry(
            RetryError::Http {
                status: StatusCode::TOO_MANY_REQUESTS,
                retry_hint: None,
            },
            POLICY,
            1,
        );
        let third = evaluate_retry(
            RetryError::Http {
                status: StatusCode::TOO_MANY_REQUESTS,
                retry_hint: None,
            },
            POLICY,
            2,
        );

        assert_eq!(first.delay_ms, 1_000);
        assert_eq!(second.delay_ms, 2_000);
        assert_eq!(third.delay_ms, 4_000);
        assert!(!first.used_hint);
        assert!(!second.used_hint);
        assert!(!third.used_hint);
    }

    #[test]
    fn respects_max_delay_cap() {
        let policy = RetryPolicy {
            base_delay: Duration::from_secs(4),
            max_delay: Duration::from_secs(10),
            max_retries: 5,
        };

        let decision = evaluate_retry(
            RetryError::Http {
                status: StatusCode::TOO_MANY_REQUESTS,
                retry_hint: None,
            },
            policy,
            3,
        );

        assert_eq!(decision.delay_ms, 10_000);
    }

    #[test]
    fn non_retryable_errors_fail_fast() {
        let decision = evaluate_retry(
            RetryError::Http {
                status: StatusCode::BAD_REQUEST,
                retry_hint: None,
            },
            POLICY,
            0,
        );

        assert_eq!(decision.should_retry, false);
    }

    #[test]
    fn parse_retry_after_seconds_header() {
        let duration = parse_retry_after("120", SystemTime::now()).unwrap();
        assert_eq!(duration, Duration::from_secs(120));
    }

    #[test]
    fn parse_retry_after_http_date() {
        let now = SystemTime::now();
        let later = now + Duration::from_secs(30);
        let header = httpdate::fmt_http_date(later);
        let parsed = parse_retry_after(&header, now).unwrap();
        assert_eq!(parsed.as_secs(), 30);
    }

    #[test]
    fn parse_gemini_retry_delay_seconds() {
        let parsed = parse_gemini_retry_delay("1.5s").unwrap();
        assert_eq!(parsed.as_millis(), 1500);
    }
}
