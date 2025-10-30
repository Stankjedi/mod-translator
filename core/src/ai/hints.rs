use httpdate::parse_http_date;
use reqwest::header::{HeaderMap, HeaderValue, RETRY_AFTER};
use serde_json::Value;
use std::time::{Duration, SystemTime};

pub const MAX_SERVER_HINT_WINDOW: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryHintSource {
    RetryAfterHeader,
    GeminiRetryInfo,
}

impl RetryHintSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            RetryHintSource::RetryAfterHeader => "retry-after",
            RetryHintSource::GeminiRetryInfo => "gemini-retry-info",
        }
    }
}

#[derive(Debug, Clone)]
pub struct RetryHint {
    pub delay: Duration,
    pub source: RetryHintSource,
    pub raw_value: Option<String>,
}

impl RetryHint {
    pub fn clamped_delay(&self) -> Duration {
        self.delay.min(MAX_SERVER_HINT_WINDOW)
    }

    pub fn raw_value(&self) -> Option<&str> {
        self.raw_value.as_deref()
    }
}

#[derive(Debug, Default)]
pub struct GeminiErrorHints {
    pub retry_hint: Option<RetryHint>,
    pub quota_failure: bool,
}

pub fn parse_retry_after_header(headers: &HeaderMap) -> Option<RetryHint> {
    let value = headers.get(RETRY_AFTER)?;
    parse_retry_after_value(value)
}

fn parse_retry_after_value(value: &HeaderValue) -> Option<RetryHint> {
    let raw = value.to_str().ok()?.trim();
    if raw.is_empty() {
        return None;
    }

    if let Ok(delta) = raw.parse::<u64>() {
        return Some(RetryHint {
            delay: Duration::from_secs(delta),
            source: RetryHintSource::RetryAfterHeader,
            raw_value: Some(raw.to_string()),
        });
    }

    if let Ok(when) = parse_http_date(raw) {
        if let Ok(delta) = when.duration_since(SystemTime::now()) {
            return Some(RetryHint {
                delay: delta,
                source: RetryHintSource::RetryAfterHeader,
                raw_value: Some(raw.to_string()),
            });
        }
    }

    None
}

pub fn parse_gemini_error_hints(body: &str) -> GeminiErrorHints {
    let mut hints = GeminiErrorHints::default();
    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return hints;
    };

    let Some(error) = value.get("error") else {
        return hints;
    };

    let Some(details) = error.get("details").and_then(|value| value.as_array()) else {
        return hints;
    };

    for detail in details {
        let type_url = detail.get("@type").and_then(Value::as_str).unwrap_or("");
        if type_url.ends_with("RetryInfo") {
            if let Some(delay_value) = detail.get("retryDelay") {
                if let Some(duration) = parse_gemini_duration(delay_value) {
                    if hints.retry_hint.is_none() {
                        let raw = extract_raw_string(delay_value);
                        hints.retry_hint = Some(RetryHint {
                            delay: duration,
                            source: RetryHintSource::GeminiRetryInfo,
                            raw_value: raw,
                        });
                    }
                }
            }
        } else if type_url.ends_with("QuotaFailure") {
            if detail
                .get("violations")
                .and_then(Value::as_array)
                .map(|items| !items.is_empty())
                .unwrap_or(true)
            {
                hints.quota_failure = true;
            }
        }
    }

    hints
}

fn parse_gemini_duration(value: &Value) -> Option<Duration> {
    if let Some(text) = value.as_str() {
        return parse_duration_string(text);
    }

    if let Some(object) = value.as_object() {
        let seconds = object.get("seconds").and_then(parse_i64_field).unwrap_or(0);
        let nanos = object.get("nanos").and_then(parse_i64_field).unwrap_or(0);
        if seconds < 0 || nanos < 0 {
            return None;
        }

        let secs: u64 = seconds.try_into().ok()?;
        let nanos: u32 = nanos.try_into().ok()?;
        let mut duration = Duration::from_secs(secs);
        duration += Duration::from_nanos(u64::from(nanos));
        return Some(duration);
    }

    None
}

fn parse_i64_field(value: &Value) -> Option<i64> {
    if let Some(number) = value.as_i64() {
        return Some(number);
    }
    let text = value.as_str()?;
    text.parse::<i64>().ok()
}

fn parse_duration_string(input: &str) -> Option<Duration> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let without_suffix = trimmed.strip_suffix('s').unwrap_or(trimmed);
    let seconds = without_suffix.parse::<f64>().ok()?;
    if !seconds.is_finite() || seconds.is_sign_negative() {
        return None;
    }

    Some(Duration::from_secs_f64(seconds))
}

fn extract_raw_string(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        return Some(text.to_string());
    }
    Some(value.to_string())
}
