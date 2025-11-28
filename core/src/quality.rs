use once_cell::sync::Lazy;
use regex::Regex;
use serde::Serialize;

static PLACEHOLDER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(\{\w+\}|\{\d+\}|%\d*\$?[sd]|%s|%d|\$[A-Z0-9_]+\$|\{Pawn_[^}]+\}|\$\{[^}]+\})")
        .expect("valid placeholder regex")
});

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentValidationResult {
    pub placeholder_parity_ok: bool,
    pub pipe_parity_ok: bool,
    pub length_ratio: f32,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

impl SegmentValidationResult {
    pub fn is_pass(&self) -> bool {
        self.errors.is_empty()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SegmentLimits {
    pub warn_ratio: f32,
    pub max_ratio: f32,
    pub max_length: Option<usize>,
}

impl Default for SegmentLimits {
    fn default() -> Self {
        Self {
            warn_ratio: 1.8,
            max_ratio: 3.0,
            max_length: None,
        }
    }
}

pub fn validate_segment(
    source: &str,
    candidate: &str,
    limits: &SegmentLimits,
) -> SegmentValidationResult {
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    let (placeholder_parity_ok, placeholder_message) = compare_placeholders(source, candidate);
    if let Some(message) = placeholder_message {
        errors.push(message);
    }

    let (pipe_parity_ok, pipe_message) = compare_pipe_count(source, candidate);
    if let Some(message) = pipe_message {
        errors.push(message);
    }

    let length_ratio = compute_length_ratio(source, candidate);
    if let Some(max_len) = limits.max_length {
        if candidate.chars().count() > max_len {
            errors.push(format!(
                "번역 결과가 허용된 길이({})를 초과했습니다.",
                max_len
            ));
        }
    }

    if length_ratio > limits.max_ratio {
        errors.push(format!(
            "번역 결과가 원문 대비 너무 깁니다 (비율 {:.2}).",
            length_ratio
        ));
    } else if length_ratio > limits.warn_ratio {
        warnings.push(format!(
            "번역 결과 길이 비율 {:.2}가 경고 임계값을 초과했습니다.",
            length_ratio
        ));
    }

    SegmentValidationResult {
        placeholder_parity_ok,
        pipe_parity_ok,
        length_ratio,
        warnings,
        errors,
    }
}

fn compute_length_ratio(source: &str, candidate: &str) -> f32 {
    let source_len = source.chars().count().max(1) as f32;
    let candidate_len = candidate.chars().count() as f32;
    (candidate_len / source_len).max(0.0)
}

fn compare_pipe_count(source: &str, candidate: &str) -> (bool, Option<String>) {
    let source_count = source.matches('|').count();
    let candidate_count = candidate.matches('|').count();
    if source_count == candidate_count {
        (true, None)
    } else {
        (
            false,
            Some(format!(
                "파이프(|) 개수가 일치하지 않습니다. 원본: {}, 번역: {}",
                source_count, candidate_count
            )),
        )
    }
}

fn compare_placeholders(source: &str, candidate: &str) -> (bool, Option<String>) {
    let source_map = collect_placeholder_counts(source);
    let candidate_map = collect_placeholder_counts(candidate);
    if source_map == candidate_map {
        (true, None)
    } else {
        (false, Some("플레이스홀더 집합이 일치하지 않습니다.".into()))
    }
}

fn collect_placeholder_counts(input: &str) -> std::collections::BTreeMap<String, usize> {
    let mut counts = std::collections::BTreeMap::new();
    for capture in PLACEHOLDER_REGEX.captures_iter(input) {
        if let Some(m) = capture.get(0) {
            let entry = counts.entry(m.as_str().to_string()).or_insert(0);
            *entry += 1;
        }
    }
    counts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_placeholder_success() {
        let limits = SegmentLimits::default();
        let result = validate_segment("Hello {0}", "안녕 {0}", &limits);
        assert!(result.is_pass());
    }

    #[test]
    fn detect_pipe_mismatch() {
        let limits = SegmentLimits::default();
        let result = validate_segment("A|B|C", "A|B", &limits);
        assert!(!result.is_pass());
        assert!(!result.pipe_parity_ok);
    }

    #[test]
    fn detect_length_warning_and_error() {
        let mut limits = SegmentLimits::default();
        limits.max_ratio = 1.5; // Set lower ratio so 2x length fails
        limits.warn_ratio = 1.2;
        // "abc" (3 chars) -> "abcdefgh" (8 chars) = ratio 2.67 > 1.5 max
        let result = validate_segment("abc", "abcdefgh", &limits);
        assert!(!result.is_pass());
        assert!(!result.errors.is_empty());
    }
}
