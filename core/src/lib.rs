pub mod ai;
pub mod archive;
pub mod backup;
pub mod codex_spec_tests;
pub mod config;
pub mod encoding;
pub mod format_validator;
pub mod formats;
pub mod job;
mod jobs;
mod library;
pub mod llm_guards;
pub mod math_units;
pub mod pipeline;
pub mod placeholder_validator;
pub mod policy;
pub mod profiles;
pub mod protector;
pub mod quality;
pub mod scanner;
pub mod scanners;
mod steam;
pub mod text_extractor;
mod time;
pub mod tone_analyzer;
pub mod translate;
mod validation;
pub mod validation_logger;
pub mod validator;

#[cfg(test)]
mod integration_tests;

pub use archive::{
    is_archive_file, scan_archive, ArchiveEntry, ArchiveModification, ArchiveScanResult, ArchiveType,
};
pub use config::{TranslatorConfig, UiOptions, ValidatorOptions};
pub use jobs::{
    cancel_translation_job, open_output_folder, retry_translation_now, start_translation_job,
    StartTranslationJobPayload, TranslationFileInput, TranslationProgressEventPayload,
};
pub use library::{
    list_mod_files, scan_steam_library, LibraryEntry, LibraryScanDebug, LibraryScanResponse,
    LibraryScanner, LibraryWorkshopDebugEntry, ModFileDescriptor, ModFileListing,
};
pub use pipeline::PipelinePlan;
pub use placeholder_validator::{
    PlaceholderValidator, Segment, ValidationErrorCode, ValidationFailureReport, ValidatorConfig,
};
pub use policy::{default_policy_banner, PolicyBanner, PolicyProfile};
pub use protector::{ProtectionMode, Protector, ProtectedFragment, ProtectorError};
pub use steam::{detect_steam_path, SteamLocator, SteamPathResponse};
pub use validation::validate_api_key_and_list_models;
pub use validation_logger::{
    export_validation_metrics, get_validation_log_file_path, get_validation_log_path,
    get_validation_metrics, init_validation_logging, reset_validation_metrics, validation_logger,
    ValidationLogEntry, ValidationLogger, ValidationMetrics, ValidationOutcome,
};
