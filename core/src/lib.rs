pub mod ai;
pub mod backup;
pub mod config;
pub mod encoding;
pub mod formats;
mod jobs;
mod library;
pub mod pipeline;
pub mod placeholder_validator;
pub mod policy;
pub mod profiles;
pub mod protector;
pub mod quality;
pub mod scanner;
mod steam;
mod time;
mod validation;
pub mod validation_logger;
pub mod validator;

pub use jobs::{
    cancel_translation_job, open_output_folder, retry_translation_now, start_translation_job,
    StartTranslationJobPayload, TranslationFileInput, TranslationProgressEventPayload,
};
pub use library::{
    list_mod_files, scan_steam_library, LibraryEntry, LibraryScanDebug, LibraryScanResponse,
    LibraryScanner, LibraryWorkshopDebugEntry, ModFileDescriptor, ModFileListing,
};
pub use config::{TranslatorConfig, ValidatorOptions, UiOptions};
pub use pipeline::PipelinePlan;
pub use placeholder_validator::{
    PlaceholderValidator, Segment, ValidationErrorCode, ValidationFailureReport,
    ValidatorConfig,
};
pub use policy::{default_policy_banner, PolicyBanner, PolicyProfile};
pub use steam::{detect_steam_path, SteamLocator, SteamPathResponse};
pub use validation::validate_api_key_and_list_models;
pub use validation_logger::{
    validation_logger, init_validation_logging, get_validation_log_path,
    ValidationLogger, ValidationMetrics, ValidationLogEntry,
    get_validation_metrics, reset_validation_metrics, export_validation_metrics,
    get_validation_log_file_path,
};
