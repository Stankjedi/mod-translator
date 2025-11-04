/// File scanning with format detection and exclusion rules
use crate::formats::{FileFormat, get_handler};
use std::path::{Path, PathBuf};
use std::fs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfig {
    /// Include paths (default: Languages/, locale/, i18n/, strings/, text/)
    #[serde(default = "default_include_paths")]
    pub include_paths: Vec<String>,
    
    /// Exclude patterns
    #[serde(default = "default_exclude_patterns")]
    pub exclude_patterns: Vec<String>,
    
    /// Maximum file size in bytes (default: 20MB)
    #[serde(default = "default_max_size")]
    pub max_file_size: usize,
    
    /// Binary detection threshold (% non-ASCII)
    #[serde(default = "default_binary_threshold")]
    pub binary_threshold: f32,
}

fn default_include_paths() -> Vec<String> {
    vec![
        "Languages/".to_string(),
        "locale/".to_string(),
        "i18n/".to_string(),
        "strings/".to_string(),
        "text/".to_string(),
    ]
}

fn default_exclude_patterns() -> Vec<String> {
    vec![
        "*.dll".to_string(),
        "*.exe".to_string(),
        "*.so".to_string(),
        "*.png".to_string(),
        "*.jpg".to_string(),
        "*.dds".to_string(),
        "*.ogg".to_string(),
        "*.wav".to_string(),
        "*.mp3".to_string(),
        "*.mesh".to_string(),
        "*.bundle".to_string(),
        "*.asset".to_string(),
        "*.esp".to_string(),
        "*.esm".to_string(),
        "*.bsa".to_string(),
    ]
}

fn default_max_size() -> usize {
    20 * 1024 * 1024 // 20MB
}

fn default_binary_threshold() -> f32 {
    0.20 // 20% non-ASCII
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            include_paths: default_include_paths(),
            exclude_patterns: default_exclude_patterns(),
            max_file_size: default_max_size(),
            binary_threshold: default_binary_threshold(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedFile {
    pub path: PathBuf,
    pub format: FileFormat,
    pub size: u64,
    pub relative_path: String,
}

#[derive(Debug)]
pub struct FileScanner {
    config: ScanConfig,
}

impl FileScanner {
    pub fn new(config: ScanConfig) -> Self {
        Self { config }
    }
    
    /// Scan a directory for translatable files
    pub fn scan(&self, root: &Path) -> Result<Vec<ScannedFile>, std::io::Error> {
        let mut files = Vec::new();
        self.scan_recursive(root, root, &mut files)?;
        Ok(files)
    }
    
    fn scan_recursive(
        &self,
        root: &Path,
        current: &Path,
        files: &mut Vec<ScannedFile>,
    ) -> Result<(), std::io::Error> {
        for entry in fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                self.scan_recursive(root, &path, files)?;
            } else if path.is_file() {
                if let Some(scanned) = self.process_file(root, &path)? {
                    files.push(scanned);
                }
            }
        }
        Ok(())
    }
    
    fn process_file(
        &self,
        root: &Path,
        path: &Path,
    ) -> Result<Option<ScannedFile>, std::io::Error> {
        let metadata = fs::metadata(path)?;
        let size = metadata.len();
        
        // Check size limit
        if size > self.config.max_file_size as u64 {
            return Ok(None);
        }
        
        // Check exclusion patterns
        let file_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        
        if self.is_excluded(file_name) {
            return Ok(None);
        }
        
        // Check if binary
        if self.is_binary(path)? {
            return Ok(None);
        }
        
        // Detect format
        let format = FileFormat::from_path(path);
        if format == FileFormat::Unknown {
            return Ok(None);
        }
        
        // Check if handler exists
        if get_handler(format).is_none() {
            return Ok(None);
        }
        
        let relative_path = path.strip_prefix(root)
            .ok()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();
        
        Ok(Some(ScannedFile {
            path: path.to_path_buf(),
            format,
            size,
            relative_path,
        }))
    }
    
    fn is_excluded(&self, filename: &str) -> bool {
        for pattern in &self.config.exclude_patterns {
            if pattern.starts_with("*.") {
                let ext = &pattern[2..];
                if filename.ends_with(ext) {
                    return true;
                }
            } else if filename.contains(pattern) {
                return true;
            }
        }
        false
    }
    
    fn is_binary(&self, path: &Path) -> Result<bool, std::io::Error> {
        let content = fs::read(path)?;
        
        // Check first 8KB for binary content
        let sample_size = content.len().min(8192);
        let sample = &content[..sample_size];
        
        let non_ascii = sample.iter()
            .filter(|&&b| b < 32 && b != b'\n' && b != b'\r' && b != b'\t')
            .count();
        
        let ratio = non_ascii as f32 / sample_size as f32;
        Ok(ratio > self.config.binary_threshold)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn scans_json_files() {
        let dir = TempDir::new().unwrap();
        let json_path = dir.path().join("test.json");
        fs::write(&json_path, r#"{"message": "Hello"}"#).unwrap();
        
        let scanner = FileScanner::new(ScanConfig::default());
        let files = scanner.scan(dir.path()).unwrap();
        
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].format, FileFormat::Json);
    }
    
    #[test]
    fn excludes_binary_files() {
        let dir = TempDir::new().unwrap();
        let bin_path = dir.path().join("test.dll");
        fs::write(&bin_path, vec![0u8; 100]).unwrap();
        
        let scanner = FileScanner::new(ScanConfig::default());
        let files = scanner.scan(dir.path()).unwrap();
        
        assert_eq!(files.len(), 0);
    }
}
