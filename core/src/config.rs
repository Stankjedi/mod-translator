/// Configuration for the translation system
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidatorOptions {
    pub enable_autofix: bool,
    pub retry_on_fail: bool,
    pub retry_limit: usize,
    pub strict_pairing: bool,
    pub preserve_percent_binding: bool,
    pub report_download: bool,
    pub jsonl_logging: bool,
}

impl Default for ValidatorOptions {
    fn default() -> Self {
        Self {
            enable_autofix: true,
            retry_on_fail: true,
            retry_limit: 1,
            strict_pairing: true,
            preserve_percent_binding: true,
            report_download: true,
            jsonl_logging: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiOptions {
    pub show_failed_line: bool,
    pub allow_clipboard_copy: bool,
    pub allow_autofix_retry: bool,
}

impl Default for UiOptions {
    fn default() -> Self {
        Self {
            show_failed_line: true,
            allow_clipboard_copy: true,
            allow_autofix_retry: true,
        }
    }
}

/// 번역 제외 패턴 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoreOptions {
    /// 사용자 정의 무시 패턴 (gitignore 형식)
    #[serde(default)]
    pub patterns: Vec<String>,
    
    /// .modtranslatorignore 파일 사용 여부
    #[serde(default = "default_true")]
    pub use_ignore_file: bool,
    
    /// 기본 무시 패턴 사용 여부 (바이너리, 미디어 파일 등)
    #[serde(default = "default_true")]
    pub use_default_patterns: bool,
}

fn default_true() -> bool {
    true
}

impl Default for IgnoreOptions {
    fn default() -> Self {
        Self {
            patterns: Vec::new(),
            use_ignore_file: true,
            use_default_patterns: true,
        }
    }
}

impl IgnoreOptions {
    /// .modtranslatorignore 파일에서 패턴 로드
    pub fn load_ignore_file<P: AsRef<Path>>(path: P) -> Result<Vec<String>, std::io::Error> {
        let content = fs::read_to_string(path)?;
        Ok(parse_ignore_patterns(&content))
    }
    
    /// 모든 패턴 수집 (사용자 정의 + 파일 + 기본)
    pub fn collect_patterns<P: AsRef<Path>>(&self, mod_root: P) -> Vec<String> {
        let mut patterns = Vec::new();
        
        // 기본 패턴
        if self.use_default_patterns {
            patterns.extend(default_ignore_patterns());
        }
        
        // .modtranslatorignore 파일에서 로드
        if self.use_ignore_file {
            let ignore_path = mod_root.as_ref().join(".modtranslatorignore");
            if let Ok(file_patterns) = Self::load_ignore_file(&ignore_path) {
                patterns.extend(file_patterns);
            }
        }
        
        // 사용자 정의 패턴
        patterns.extend(self.patterns.clone());
        
        patterns
    }
}

/// gitignore 형식의 패턴 파싱
pub fn parse_ignore_patterns(content: &str) -> Vec<String> {
    content
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.to_string())
        .collect()
}

/// 기본 무시 패턴 (바이너리, 미디어, 개발 파일)
pub fn default_ignore_patterns() -> Vec<String> {
    vec![
        // 바이너리 파일
        "*.dll".to_string(),
        "*.exe".to_string(),
        "*.so".to_string(),
        "*.dylib".to_string(),
        // 이미지
        "*.png".to_string(),
        "*.jpg".to_string(),
        "*.jpeg".to_string(),
        "*.gif".to_string(),
        "*.bmp".to_string(),
        "*.dds".to_string(),
        "*.tga".to_string(),
        "*.psd".to_string(),
        // 오디오
        "*.ogg".to_string(),
        "*.wav".to_string(),
        "*.mp3".to_string(),
        "*.flac".to_string(),
        // 3D/게임 에셋
        "*.mesh".to_string(),
        "*.bundle".to_string(),
        "*.asset".to_string(),
        "*.unity3d".to_string(),
        // Bethesda 형식
        "*.esp".to_string(),
        "*.esm".to_string(),
        "*.bsa".to_string(),
        "*.ba2".to_string(),
        // 개발 폴더
        ".git/".to_string(),
        ".svn/".to_string(),
        "node_modules/".to_string(),
        "__pycache__/".to_string(),
        // 테스트/문서
        "test/".to_string(),
        "tests/".to_string(),
        "docs/".to_string(),
        "README*".to_string(),
        "LICENSE*".to_string(),
        "CHANGELOG*".to_string(),
    ]
}

/// 경로가 패턴과 일치하는지 확인
pub fn matches_ignore_pattern(path: &str, pattern: &str) -> bool {
    let path_lower = path.to_lowercase();
    let pattern_lower = pattern.to_lowercase();
    
    // 디렉토리 패턴 (끝이 /로 끝남)
    if pattern_lower.ends_with('/') {
        let dir_pattern = &pattern_lower[..pattern_lower.len() - 1];
        return path_lower.contains(&format!("/{}/", dir_pattern))
            || path_lower.contains(&format!("\\{}\\", dir_pattern))
            || path_lower.starts_with(&format!("{}/", dir_pattern))
            || path_lower.starts_with(&format!("{}\\", dir_pattern));
    }
    
    // 와일드카드 패턴 (*.ext)
    if pattern_lower.starts_with("*.") {
        let ext = &pattern_lower[1..]; // .ext 포함
        return path_lower.ends_with(ext);
    }
    
    // 파일명 와일드카드 (README*)
    if pattern_lower.ends_with('*') {
        let prefix = &pattern_lower[..pattern_lower.len() - 1];
        let filename = path.rsplit(&['/', '\\']).next().unwrap_or(path);
        return filename.to_lowercase().starts_with(prefix);
    }
    
    // 정확한 일치 또는 포함
    path_lower.contains(&pattern_lower)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatorConfig {
    pub validator: ValidatorOptions,
    pub ui: UiOptions,
    #[serde(default)]
    pub ignore: IgnoreOptions,
}

impl Default for TranslatorConfig {
    fn default() -> Self {
        Self {
            validator: ValidatorOptions::default(),
            ui: UiOptions::default(),
            ignore: IgnoreOptions::default(),
        }
    }
}

impl TranslatorConfig {
    /// Load configuration from YAML file
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file: {}", e))?;
        
        serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse config: {}", e))
    }

    /// Save configuration to YAML file
    pub fn to_yaml_file<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize config: {}", e))?;
        
        fs::write(path, content)
            .map_err(|e| format!("Failed to write config file: {}", e))
    }

    /// Load from JSON string (for UI integration)
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse JSON config: {}", e))
    }

    /// Convert to JSON string (for UI integration)
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize to JSON: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = TranslatorConfig::default();
        assert!(config.validator.enable_autofix);
        assert_eq!(config.validator.retry_limit, 1);
        assert!(config.ui.show_failed_line);
    }

    #[test]
    fn test_json_serialization() {
        let config = TranslatorConfig::default();
        let json = config.to_json().unwrap();
        let deserialized = TranslatorConfig::from_json(&json).unwrap();
        
        assert_eq!(config.validator.enable_autofix, deserialized.validator.enable_autofix);
        assert_eq!(config.validator.retry_limit, deserialized.validator.retry_limit);
    }
    
    #[test]
    fn test_parse_ignore_patterns() {
        let content = r#"
# Comment line
*.dll
*.exe

test/
docs/
README*
"#;
        let patterns = parse_ignore_patterns(content);
        assert_eq!(patterns.len(), 5);
        assert!(patterns.contains(&"*.dll".to_string()));
        assert!(patterns.contains(&"test/".to_string()));
        assert!(patterns.contains(&"README*".to_string()));
    }
    
    #[test]
    fn test_matches_ignore_pattern() {
        // Extension patterns
        assert!(matches_ignore_pattern("test.dll", "*.dll"));
        assert!(matches_ignore_pattern("path/to/file.exe", "*.exe"));
        assert!(!matches_ignore_pattern("test.json", "*.dll"));
        
        // Directory patterns
        assert!(matches_ignore_pattern("test/file.json", "test/"));
        assert!(matches_ignore_pattern("path/test/file.json", "test/"));
        assert!(!matches_ignore_pattern("testing/file.json", "test/"));
        
        // Prefix wildcard patterns
        assert!(matches_ignore_pattern("README.md", "README*"));
        assert!(matches_ignore_pattern("README-KO.md", "README*"));
        assert!(!matches_ignore_pattern("NOTREADME.md", "README*"));
        
        // Case insensitive
        assert!(matches_ignore_pattern("TEST.DLL", "*.dll"));
        assert!(matches_ignore_pattern("Readme.md", "README*"));
    }
    
    #[test]
    fn test_default_ignore_patterns() {
        let patterns = default_ignore_patterns();
        
        // Should include binary files
        assert!(patterns.contains(&"*.dll".to_string()));
        assert!(patterns.contains(&"*.exe".to_string()));
        
        // Should include media files
        assert!(patterns.contains(&"*.png".to_string()));
        assert!(patterns.contains(&"*.ogg".to_string()));
        
        // Should include dev directories
        assert!(patterns.contains(&".git/".to_string()));
        assert!(patterns.contains(&"node_modules/".to_string()));
    }
}
