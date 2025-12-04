//! ZIP/JAR 아카이브 파일 처리 모듈
//!
//! 마인크래프트 모드(.jar) 및 일반 ZIP 아카이브 내부의
//! 언어 파일을 읽고, 번역 후 재패키징하는 기능을 제공합니다.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use zip::read::ZipArchive;
use zip::write::{FileOptions, ZipWriter};
use zip::CompressionMethod;

/// 아카이브 처리 결과 에러 타입
#[derive(Debug, thiserror::Error)]
pub enum ArchiveError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("ZIP error: {0}")]
    Zip(#[from] zip::result::ZipError),

    #[error("Archive not found: {0}")]
    NotFound(String),

    #[error("Invalid archive format: {0}")]
    InvalidFormat(String),

    #[error("Entry not found in archive: {0}")]
    EntryNotFound(String),
}

pub type ArchiveResult<T> = Result<T, ArchiveError>;

/// 아카이브 내부 파일 정보
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveEntry {
    /// 아카이브 내부 경로 (예: "assets/mymod/lang/en_us.json")
    pub path: String,
    /// 파일 크기 (bytes)
    pub size: u64,
    /// 압축된 크기 (bytes)
    pub compressed_size: u64,
    /// 디렉토리 여부
    pub is_dir: bool,
}

/// 아카이브 스캔 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveScanResult {
    /// 아카이브 파일 경로
    pub archive_path: PathBuf,
    /// 발견된 언어 파일들
    pub language_files: Vec<ArchiveEntry>,
    /// 아카이브 타입 (jar, zip)
    pub archive_type: ArchiveType,
    /// 총 엔트리 수
    pub total_entries: usize,
}

/// 아카이브 타입
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArchiveType {
    Jar,
    Zip,
}

impl ArchiveType {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "jar" => Some(Self::Jar),
            "zip" => Some(Self::Zip),
            _ => None,
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Jar => "jar",
            Self::Zip => "zip",
        }
    }
}

/// 아카이브 파일인지 확인
pub fn is_archive_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| matches!(ext.to_lowercase().as_str(), "jar" | "zip"))
        .unwrap_or(false)
}

/// 아카이브 타입 감지
pub fn detect_archive_type(path: &Path) -> Option<ArchiveType> {
    path.extension()
        .and_then(|ext| ext.to_str())
        .and_then(ArchiveType::from_extension)
}

/// 언어 파일 패턴 매칭
fn is_language_file(entry_path: &str) -> bool {
    let lower = entry_path.to_lowercase();
    
    // 마인크래프트 언어 파일 패턴
    // assets/{modid}/lang/*.json
    if lower.contains("/lang/") && lower.ends_with(".json") {
        return true;
    }
    
    // 일반적인 로컬라이제이션 패턴 (시작, 중간 모두 검사)
    let localization_patterns = [
        "localization/",
        "localisation/",
        "languages/",
        "language/",
        "i18n/",
        "locale/",
        "l10n/",
        "strings/",
        "text/",
        "/localization/",
        "/localisation/",
        "/languages/",
        "/language/",
        "/i18n/",
        "/locale/",
        "/l10n/",
        "/strings/",
        "/text/",
    ];
    
    for pattern in localization_patterns {
        if lower.contains(pattern) || lower.starts_with(pattern.trim_start_matches('/')) {
            let ext = Path::new(&lower)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            
            if matches!(ext, "json" | "xml" | "yml" | "yaml" | "properties" | "cfg" | "ini" | "txt" | "lang") {
                return true;
            }
        }
    }
    
    // .lang 파일 (레거시 마인크래프트)
    if lower.ends_with(".lang") {
        return true;
    }
    
    false
}

/// 아카이브 내부 스캔
pub fn scan_archive(archive_path: &Path) -> ArchiveResult<ArchiveScanResult> {
    if !archive_path.exists() {
        return Err(ArchiveError::NotFound(archive_path.display().to_string()));
    }

    let archive_type = detect_archive_type(archive_path)
        .ok_or_else(|| ArchiveError::InvalidFormat(archive_path.display().to_string()))?;

    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;

    let total_entries = archive.len();
    let mut language_files = Vec::new();

    for i in 0..total_entries {
        let entry = archive.by_index(i)?;
        let entry_path = entry.name().to_string();
        
        if entry.is_dir() {
            continue;
        }

        if is_language_file(&entry_path) {
            language_files.push(ArchiveEntry {
                path: entry_path,
                size: entry.size(),
                compressed_size: entry.compressed_size(),
                is_dir: false,
            });
        }
    }

    Ok(ArchiveScanResult {
        archive_path: archive_path.to_path_buf(),
        language_files,
        archive_type,
        total_entries,
    })
}

/// 아카이브에서 특정 파일 내용 읽기
pub fn read_archive_entry(archive_path: &Path, entry_path: &str) -> ArchiveResult<Vec<u8>> {
    let file = File::open(archive_path)?;
    let mut archive = ZipArchive::new(file)?;

    let mut entry = archive.by_name(entry_path)
        .map_err(|_| ArchiveError::EntryNotFound(entry_path.to_string()))?;

    let mut contents = Vec::with_capacity(entry.size() as usize);
    entry.read_to_end(&mut contents)?;

    Ok(contents)
}

/// 아카이브에서 특정 파일 내용을 문자열로 읽기
pub fn read_archive_entry_string(archive_path: &Path, entry_path: &str) -> ArchiveResult<String> {
    let bytes = read_archive_entry(archive_path, entry_path)?;
    
    // UTF-8 BOM 제거
    let content = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        String::from_utf8_lossy(&bytes[3..]).to_string()
    } else {
        String::from_utf8_lossy(&bytes).to_string()
    };
    
    Ok(content)
}

/// 아카이브 수정 옵션
#[derive(Debug, Clone)]
pub struct ArchiveModification {
    /// 수정할 파일 경로 -> 새 내용
    pub updates: HashMap<String, Vec<u8>>,
    /// 새로 추가할 파일 경로 -> 내용
    pub additions: HashMap<String, Vec<u8>>,
}

impl Default for ArchiveModification {
    fn default() -> Self {
        Self::new()
    }
}

impl ArchiveModification {
    pub fn new() -> Self {
        Self {
            updates: HashMap::new(),
            additions: HashMap::new(),
        }
    }

    /// 기존 파일 업데이트 추가
    pub fn update_file(&mut self, path: impl Into<String>, content: Vec<u8>) {
        self.updates.insert(path.into(), content);
    }

    /// 기존 파일을 문자열로 업데이트
    pub fn update_file_string(&mut self, path: impl Into<String>, content: &str) {
        self.updates.insert(path.into(), content.as_bytes().to_vec());
    }

    /// 새 파일 추가
    pub fn add_file(&mut self, path: impl Into<String>, content: Vec<u8>) {
        self.additions.insert(path.into(), content);
    }

    /// 새 파일을 문자열로 추가
    pub fn add_file_string(&mut self, path: impl Into<String>, content: &str) {
        self.additions.insert(path.into(), content.as_bytes().to_vec());
    }

    pub fn is_empty(&self) -> bool {
        self.updates.is_empty() && self.additions.is_empty()
    }
}

/// 아카이브 수정 및 새 파일로 저장
/// 
/// 원본 아카이브를 읽어서 수정사항을 적용한 후 새 파일로 저장합니다.
/// 원본 파일은 변경되지 않습니다.
pub fn modify_archive(
    source_path: &Path,
    output_path: &Path,
    modifications: &ArchiveModification,
) -> ArchiveResult<()> {
    if modifications.is_empty() {
        // 수정사항이 없으면 단순 복사
        fs::copy(source_path, output_path)?;
        return Ok(());
    }

    let source_file = File::open(source_path)?;
    let mut source_archive = ZipArchive::new(source_file)?;

    // 임시 파일에 작성 후 최종 위치로 이동
    let output_dir = output_path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(output_dir)?;
    
    let output_file = File::create(output_path)?;
    let mut writer = ZipWriter::new(output_file);

    // 압축 옵션 설정
    let options = FileOptions::<()>::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);

    // 기존 엔트리 복사 (수정할 항목은 새 내용으로 대체)
    for i in 0..source_archive.len() {
        let mut entry = source_archive.by_index(i)?;
        let entry_name = entry.name().to_string();

        if entry.is_dir() {
            writer.add_directory(&entry_name, options.clone())?;
            continue;
        }

        // 수정 대상인지 확인
        if let Some(new_content) = modifications.updates.get(&entry_name) {
            writer.start_file(&entry_name, options.clone())?;
            writer.write_all(new_content)?;
        } else {
            // 원본 그대로 복사
            let mut content = Vec::with_capacity(entry.size() as usize);
            entry.read_to_end(&mut content)?;
            writer.start_file(&entry_name, options.clone())?;
            writer.write_all(&content)?;
        }
    }

    // 새 파일 추가
    for (path, content) in &modifications.additions {
        // 이미 존재하는 파일이 아닌지 확인
        let exists = (0..source_archive.len()).any(|i| {
            source_archive.by_index(i)
                .map(|e| e.name() == path)
                .unwrap_or(false)
        });

        if !exists {
            writer.start_file(path, options.clone())?;
            writer.write_all(content)?;
        }
    }

    writer.finish()?;
    Ok(())
}

/// 아카이브 내부에 번역된 언어 파일을 추가/업데이트하고 원본 백업 후 덮어쓰기
pub fn update_archive_with_translations(
    archive_path: &Path,
    translations: HashMap<String, String>,
    backup_dir: Option<&Path>,
) -> ArchiveResult<PathBuf> {
    // 백업 생성
    if let Some(backup_base) = backup_dir {
        let file_name = archive_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("archive");
        let backup_path = backup_base.join(format!("{}.backup", file_name));
        fs::create_dir_all(backup_base)?;
        fs::copy(archive_path, &backup_path)?;
    }

    // 수정사항 준비
    let mut modifications = ArchiveModification::new();
    for (entry_path, content) in translations {
        // 기존 파일이면 update, 새 파일이면 add
        let file = File::open(archive_path)?;
        let mut archive = ZipArchive::new(file)?;
        
        let exists = archive.by_name(&entry_path).is_ok();
        
        if exists {
            modifications.update_file_string(&entry_path, &content);
        } else {
            modifications.add_file_string(&entry_path, &content);
        }
    }

    // 임시 파일에 수정된 아카이브 생성
    let temp_path = archive_path.with_extension("tmp");
    modify_archive(archive_path, &temp_path, &modifications)?;

    // 원본을 임시 파일로 교체
    fs::rename(&temp_path, archive_path)?;

    Ok(archive_path.to_path_buf())
}

/// 마인크래프트 모드 JAR에서 언어 파일 경로 생성
/// 
/// 원본: assets/{modid}/lang/en_us.json
/// 번역: assets/{modid}/lang/ko_kr.json
pub fn minecraft_lang_target_path(source_path: &str, target_lang: &str) -> Option<String> {
    // assets/modid/lang/en_us.json -> assets/modid/lang/ko_kr.json
    let path = Path::new(source_path);
    let parent = path.parent()?;
    let file_stem = path.file_stem()?.to_str()?;
    let extension = path.extension()?.to_str()?;
    
    // 언어 코드 패턴 확인 (en_us, en_US, en-us 등)
    let lang_pattern = regex::Regex::new(r"^[a-z]{2}[_-][a-z]{2}$").ok()?;
    if !lang_pattern.is_match(&file_stem.to_lowercase()) {
        return None;
    }
    
    let new_filename = format!("{}.{}", target_lang, extension);
    Some(parent.join(new_filename).to_string_lossy().replace('\\', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_archive_file() {
        assert!(is_archive_file(Path::new("mod.jar")));
        assert!(is_archive_file(Path::new("archive.zip")));
        assert!(is_archive_file(Path::new("test.ZIP")));
        assert!(is_archive_file(Path::new("mod.JAR")));
        assert!(!is_archive_file(Path::new("file.txt")));
        assert!(!is_archive_file(Path::new("file.json")));
    }

    #[test]
    fn test_archive_type_detection() {
        assert_eq!(detect_archive_type(Path::new("mod.jar")), Some(ArchiveType::Jar));
        assert_eq!(detect_archive_type(Path::new("archive.zip")), Some(ArchiveType::Zip));
        assert_eq!(detect_archive_type(Path::new("file.txt")), None);
    }

    #[test]
    fn test_is_language_file() {
        // 마인크래프트 패턴
        assert!(is_language_file("assets/mymod/lang/en_us.json"));
        assert!(is_language_file("assets/mymod/lang/ko_kr.json"));
        
        // 일반 로컬라이제이션 패턴
        assert!(is_language_file("localization/english.json"));
        assert!(is_language_file("languages/korean.xml"));
        assert!(is_language_file("i18n/messages.properties"));
        
        // 레거시 .lang 파일
        assert!(is_language_file("lang/en_US.lang"));
        
        // 비언어 파일
        assert!(!is_language_file("assets/mymod/textures/block.png"));
        assert!(!is_language_file("config/settings.json"));
    }

    #[test]
    fn test_minecraft_lang_target_path() {
        assert_eq!(
            minecraft_lang_target_path("assets/mymod/lang/en_us.json", "ko_kr"),
            Some("assets/mymod/lang/ko_kr.json".to_string())
        );
        assert_eq!(
            minecraft_lang_target_path("assets/testmod/lang/en_US.json", "ko_kr"),
            Some("assets/testmod/lang/ko_kr.json".to_string())
        );
        // 언어 코드가 아닌 파일명
        assert_eq!(
            minecraft_lang_target_path("assets/mymod/lang/messages.json", "ko_kr"),
            None
        );
    }

    #[test]
    fn test_archive_modification() {
        let mut mods = ArchiveModification::new();
        assert!(mods.is_empty());
        
        mods.update_file_string("test.txt", "content");
        assert!(!mods.is_empty());
        
        mods.add_file_string("new.txt", "new content");
        assert_eq!(mods.updates.len(), 1);
        assert_eq!(mods.additions.len(), 1);
    }
}
