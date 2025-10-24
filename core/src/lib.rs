pub mod ai;
mod jobs;
mod library;
pub mod pipeline;
pub mod policy;
mod steam;

use jobs::{TranslationJobRequest, TranslationJobStatus, TranslationOrchestrator};
use library::{LibraryEntry, LibraryScanner};
use serde::{Deserialize, Serialize};
use steam::SteamLocator;

pub use ai::TranslatorKind;
pub use jobs::{JobState, TranslationJobRequest as JobRequest};
pub use pipeline::PipelinePlan;
pub use policy::{default_policy_banner, PolicyBanner, PolicyProfile};

#[derive(Debug, Serialize, Clone)]
pub struct SteamPathResponse {
    pub path: Option<String>,
    pub note: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct LibraryScanResponse {
    pub libraries: Vec<LibraryEntry>,
    pub policy_banner: PolicyBanner,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranslationJobSummary {
    pub request: TranslationJobRequest,
    pub status: TranslationJobStatus,
}

#[tauri::command]
pub fn detect_steam_path() -> Result<SteamPathResponse, String> {
    let locator = SteamLocator::new();
    let discovered = locator.discover_path();

    let note = if discovered.is_some() {
        "Steam 설치 경로를 자동으로 감지했습니다.".to_string()
    } else {
        "Steam 경로를 자동으로 찾지 못했습니다. 직접 경로를 입력해 주세요.".to_string()
    };

    Ok(SteamPathResponse {
        path: discovered.map(|path| path.to_string_lossy().to_string()),
        note,
    })
}

#[tauri::command]
pub fn scan_steam_library(explicit_path: Option<String>) -> Result<LibraryScanResponse, String> {
    let locator = SteamLocator::new();
    let scanner = LibraryScanner::new();

    let primary_path = explicit_path
        .and_then(|value| {
            if value.trim().is_empty() {
                None
            } else {
                Some(value.into())
            }
        })
        .or_else(|| {
            locator
                .discover_path()
                .map(|path| path.to_string_lossy().to_string())
        });

    let candidates = locator.library_candidates(primary_path.as_deref());
    let libraries = scanner.scan(&candidates)?;

    Ok(LibraryScanResponse {
        libraries,
        policy_banner: policy::default_policy_banner(),
    })
}

#[tauri::command]
pub fn start_translation_job(
    request: TranslationJobRequest,
) -> Result<TranslationJobStatus, String> {
    let orchestrator = TranslationOrchestrator::new();
    orchestrator
        .start_job(request)
        .map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn responses_are_serializable() {
        let path = detect_steam_path().unwrap();
        serde_json::to_string(&path).expect("steam path should serialize");

        let libraries = scan_steam_library(None).unwrap();
        serde_json::to_string(&libraries).expect("library scan should serialize");

        let request = TranslationJobRequest {
            mod_id: "example-mod".into(),
            mod_name: Some("Example Mod".into()),
            translator: TranslatorKind::Gpt,
            source_language: "en".into(),
            target_language: "jp".into(),
        };
        let status = start_translation_job(request).unwrap();
        serde_json::to_string(&status).expect("job status should serialize");
    }
}
