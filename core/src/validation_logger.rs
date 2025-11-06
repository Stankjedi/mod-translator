/// Logging and metrics for placeholder validation
use crate::placeholder_validator::{ValidationErrorCode, ValidationFailureReport};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Log entry for a validation failure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationLogEntry {
    pub timestamp: DateTime<Utc>,
    pub code: ValidationErrorCode,
    pub file: String,
    pub line: u32,
    pub key: String,
    pub autofix_applied: bool,
    pub autofix_success: bool,
    pub retry_attempted: bool,
    pub retry_success: bool,
}

impl From<&ValidationFailureReport> for ValidationLogEntry {
    fn from(report: &ValidationFailureReport) -> Self {
        Self {
            timestamp: Utc::now(),
            code: report.code,
            file: report.file.clone(),
            line: report.line,
            key: report.key.clone(),
            autofix_applied: report.autofix.applied,
            autofix_success: report.autofix.applied,
            retry_attempted: report.retry.attempted,
            retry_success: report.retry.success.unwrap_or(false),
        }
    }
}

/// Outcome of a validation attempt
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ValidationOutcome {
    /// Validation passed without additional recovery
    Clean,
    /// Validation passed but required automatic recovery with warnings
    RecoveredWithWarn,
}

/// Metrics for validation operations
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ValidationMetrics {
    pub total_validations: u64,
    pub total_failures: u64,
    pub autofix_attempts: u64,
    pub autofix_successes: u64,
    pub retry_attempts: u64,
    pub retry_successes: u64,
    pub recovered_with_warn: u64,
    pub by_error_code: std::collections::HashMap<String, u64>,
}

impl ValidationMetrics {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_validation(&mut self, success: bool) {
        self.total_validations += 1;
        if !success {
            self.total_failures += 1;
        }
    }

    pub fn record_autofix(&mut self, success: bool) {
        self.autofix_attempts += 1;
        if success {
            self.autofix_successes += 1;
        }
    }

    pub fn record_retry(&mut self, success: bool) {
        self.retry_attempts += 1;
        if success {
            self.retry_successes += 1;
        }
    }

    pub fn record_recovered_with_warn(&mut self) {
        self.recovered_with_warn += 1;
    }

    pub fn record_error_code(&mut self, code: &ValidationErrorCode) {
        let code_str = format!("{:?}", code);
        *self.by_error_code.entry(code_str).or_insert(0) += 1;
    }

    pub fn failure_rate(&self) -> f64 {
        if self.total_validations == 0 {
            0.0
        } else {
            self.total_failures as f64 / self.total_validations as f64
        }
    }

    pub fn autofix_success_rate(&self) -> f64 {
        if self.autofix_attempts == 0 {
            0.0
        } else {
            self.autofix_successes as f64 / self.autofix_attempts as f64
        }
    }

    pub fn retry_success_rate(&self) -> f64 {
        if self.retry_attempts == 0 {
            0.0
        } else {
            self.retry_successes as f64 / self.retry_attempts as f64
        }
    }
}

/// Logger for validation operations
pub struct ValidationLogger {
    log_file: Mutex<Option<BufWriter<File>>>,
    metrics: Mutex<ValidationMetrics>,
}

impl ValidationLogger {
    pub fn new() -> Self {
        Self {
            log_file: Mutex::new(None),
            metrics: Mutex::new(ValidationMetrics::new()),
        }
    }

    /// Initialize logging to a file
    pub fn init_file_logging<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|e| format!("Failed to open log file: {}", e))?;

        let writer = BufWriter::new(file);
        if let Ok(mut guard) = self.log_file.lock() {
            *guard = Some(writer);
        }

        Ok(())
    }

    /// Log a validation failure
    pub fn log_failure(&self, report: &ValidationFailureReport) {
        let entry = ValidationLogEntry::from(report);

        // Write to JSONL file
        if let Ok(mut guard) = self.log_file.lock() {
            if let Some(writer) = guard.as_mut() {
                if let Ok(json) = serde_json::to_string(&entry) {
                    let _ = writeln!(writer, "{}", json);
                    let _ = writer.flush();
                }
            }
        }

        // Update metrics
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.record_validation(false);
            metrics.record_error_code(&report.code);

            if report.autofix.applied {
                // If autofix was applied, we don't have the full report, so we can't determine success
                // In real implementation, we'd track this properly
                metrics.record_autofix(false);
            }

            if report.retry.attempted {
                metrics.record_retry(report.retry.success.unwrap_or(false));
            }
        }
    }

    /// Log a validation success
    pub fn log_success(&self, outcome: ValidationOutcome) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.record_validation(true);
            if matches!(outcome, ValidationOutcome::RecoveredWithWarn) {
                metrics.record_recovered_with_warn();
            }
        }
    }

    /// Get current metrics
    pub fn get_metrics(&self) -> ValidationMetrics {
        self.metrics
            .lock()
            .map(|guard| guard.clone())
            .unwrap_or_default()
    }

    /// Reset metrics
    pub fn reset_metrics(&self) {
        if let Ok(mut guard) = self.metrics.lock() {
            *guard = ValidationMetrics::new();
        }
    }

    /// Export metrics to JSON
    pub fn export_metrics_json(&self) -> Result<String, String> {
        let metrics = self.get_metrics();
        serde_json::to_string_pretty(&metrics)
            .map_err(|e| format!("Failed to serialize metrics: {}", e))
    }
}

impl Default for ValidationLogger {
    fn default() -> Self {
        Self::new()
    }
}

/// Global validation logger instance
static VALIDATION_LOGGER: once_cell::sync::Lazy<ValidationLogger> =
    once_cell::sync::Lazy::new(ValidationLogger::new);

/// Get the global validation logger
pub fn validation_logger() -> &'static ValidationLogger {
    &VALIDATION_LOGGER
}

/// Initialize file logging for the global logger
pub fn init_validation_logging<P: AsRef<Path>>(path: P) -> Result<(), String> {
    validation_logger().init_file_logging(path)
}

/// Get log file path for current session
pub fn get_validation_log_path() -> PathBuf {
    let app_dir = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mod-translator")
        .join("logs");

    std::fs::create_dir_all(&app_dir).ok();

    let timestamp = chrono::Local::now().format("%Y%m%d");
    app_dir.join(format!("validation-{}.jsonl", timestamp))
}

/// Tauri command to get validation metrics
#[tauri::command]
pub async fn get_validation_metrics() -> Result<ValidationMetrics, String> {
    Ok(validation_logger().get_metrics())
}

/// Tauri command to reset validation metrics
#[tauri::command]
pub async fn reset_validation_metrics() -> Result<(), String> {
    validation_logger().reset_metrics();
    Ok(())
}

/// Tauri command to export metrics as JSON
#[tauri::command]
pub async fn export_validation_metrics() -> Result<String, String> {
    validation_logger().export_metrics_json()
}

/// Tauri command to get validation log file path
#[tauri::command]
pub async fn get_validation_log_file_path() -> Result<String, String> {
    Ok(get_validation_log_path().to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::placeholder_validator::{AutofixResult, RetryInfo, UiHint};

    #[test]
    fn test_metrics_recording() {
        let mut metrics = ValidationMetrics::new();

        metrics.record_validation(true);
        metrics.record_validation(false);
        metrics.record_validation(false);

        assert_eq!(metrics.total_validations, 3);
        assert_eq!(metrics.total_failures, 2);
        assert_eq!(metrics.failure_rate(), 2.0 / 3.0);
    }

    #[test]
    fn test_autofix_metrics() {
        let mut metrics = ValidationMetrics::new();

        metrics.record_autofix(true);
        metrics.record_autofix(false);

        assert_eq!(metrics.autofix_attempts, 2);
        assert_eq!(metrics.autofix_successes, 1);
        assert_eq!(metrics.autofix_success_rate(), 0.5);
    }

    #[test]
    fn test_log_entry_from_report() {
        let report = ValidationFailureReport {
            code: ValidationErrorCode::PlaceholderMismatch,
            file: "test.xml".to_string(),
            line: 10,
            key: "test_key".to_string(),
            expected_protected: vec![],
            found_protected: vec![],
            expected_format: vec![],
            found_format: vec![],
            source_line: "".to_string(),
            preprocessed_source: "".to_string(),
            candidate_line: "".to_string(),
            autofix: AutofixResult {
                applied: true,
                steps: vec![],
            },
            retry: RetryInfo {
                attempted: false,
                success: None,
            },
            ui_hint: UiHint::default(),
        };

        let entry = ValidationLogEntry::from(&report);
        assert_eq!(entry.code, ValidationErrorCode::PlaceholderMismatch);
        assert_eq!(entry.file, "test.xml");
        assert_eq!(entry.line, 10);
        assert!(entry.autofix_applied);
    }
}
