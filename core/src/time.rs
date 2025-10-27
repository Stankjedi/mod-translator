use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Represents a formatted view of a [`SystemTime`] that can be serialized
/// across the Tauri boundary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormattedTimestamp {
    /// Local-date representation in `YYYY-MM-DD` form.
    pub iso_date: String,
}

impl FormattedTimestamp {
    pub fn new(iso_date: String) -> Self {
        Self { iso_date }
    }
}

/// Convert [`SystemTime`] values to a deterministic `YYYY-MM-DD` string.
///
/// Chrono's `DateTime::from` avoids deprecated timestamp constructors and
/// ensures the conversion respects the local timezone. When the conversion is
/// not possible (for example, when the timestamp predates the UNIX epoch), a
/// user-friendly error string is returned so the caller can surface the issue
/// to the UI without panicking.
pub fn format_system_time(time: SystemTime) -> Result<FormattedTimestamp, String> {
    let datetime_utc: DateTime<Utc> = match time.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(_) => DateTime::<Utc>::from(time),
        Err(_) => return Err("시간 정보를 변환할 수 없습니다.".into()),
    };

    let datetime_local = datetime_utc.with_timezone(&Local);
    Ok(FormattedTimestamp::new(
        datetime_local.format("%Y-%m-%d").to_string(),
    ))
}
