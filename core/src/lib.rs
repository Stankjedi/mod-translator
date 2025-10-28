pub mod ai;
mod jobs;
mod library;
pub mod pipeline;
pub mod policy;
mod steam;
mod time;

pub use ai::TranslatorKind;
pub use jobs::{
    get_translation_job_status, start_translation_job, JobState, TranslationJobRequest,
    TranslationJobRequest as JobRequest, TranslationJobStatus, TranslationOrchestrator,
};
pub use library::{
    list_mod_files, scan_steam_library, LibraryEntry, LibraryScanResponse, LibraryScanner,
    ModFileDescriptor, ModFileListing,
};
pub use pipeline::PipelinePlan;
pub use policy::{default_policy_banner, PolicyBanner, PolicyProfile};
use serde::{Deserialize, Serialize};
pub use steam::{detect_steam_path, SteamLocator, SteamPathResponse};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TranslationJobSummary {
    pub request: TranslationJobRequest,
    pub status: TranslationJobStatus,
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
            source_language_guess: "en".into(),
            target_language: "jp".into(),
            selected_files: vec!["localization/example.txt".into()],
            provider_auth: crate::ai::ProviderAuth::default(),
        };
        let status = TranslationOrchestrator::new()
            .start_job(request, None)
            .expect("start translation job");
        serde_json::to_string(&status).expect("job status should serialize");
    }
}
