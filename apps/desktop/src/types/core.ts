export type LibraryStatus = "healthy" | "missing";

export interface FormattedTimestamp {
  iso_date: string;
}

export interface PolicyBanner {
  headline: string;
  message: string;
  requires_acknowledgement: boolean;
  checkbox_label: string;
  warning: string;
}

export interface PolicyProfile {
  game: string;
  redistribution_blocked: boolean;
  requires_author_permission: boolean;
  eula_reference: string | null;
  notes: string[];
}

export interface ModSummary {
  id: string;
  name: string;
  game: string;
  directory: string;
  installed_languages: string[];
  last_updated: FormattedTimestamp;
  policy: PolicyProfile;
  warnings: string[];
}

export interface LibraryEntry {
  path: string;
  status: LibraryStatus;
  mods: ModSummary[];
  workshop_root: string | null;
  notes: string[];
}

export interface CanonicalizedPathSnapshot {
  original: string;
  canonical: string | null;
  key: string | null;
  status: string;
  note?: string | null;
}

export interface DuplicateLibrarySnapshot {
  existing: string;
  duplicate: string;
}

export interface RejectedLibraryCandidate {
  path: string;
  reason: string;
}

export interface LibraryDiscoveryDebug {
  raw_candidates: string[];
  canonicalized: CanonicalizedPathSnapshot[];
  skipped_symlinks: string[];
  collapsed_duplicates: DuplicateLibrarySnapshot[];
  rejected_candidates: RejectedLibraryCandidate[];
  final_libraries: string[];
}

export interface LibraryWorkshopDebugEntry {
  library: string;
  total_candidates: number;
  unique_mods: number;
  duplicates: string[];
  skipped_symlinks: string[];
}

export interface LibraryScanDebug {
  discovery: LibraryDiscoveryDebug;
  workshop: LibraryWorkshopDebugEntry[];
}

export interface LibraryScanResponse {
  libraries: LibraryEntry[];
  policy_banner: PolicyBanner;
  debug?: LibraryScanDebug;
}

export interface SteamPathResponse {
  path: string | null;
  note: string;
}

export type JobState =
  | "pending"
  | "running"
  | "completed"
  | "failed"
  | "canceled"
  | "partial_success";

export type ProviderId = "gemini" | "gpt" | "claude" | "grok";

export type RetryableErrorCode =
  | "RATE_LIMITED"
  | "NETWORK_TRANSIENT"
  | "SERVER_TRANSIENT"
  | "NETWORK"
  | "TIMEOUT"
  | "SERVER_5XX"
  | "BAD_GATEWAY"
  | "SERVICE_UNAVAILABLE";

export interface RetryPolicy {
  maxAttempts: number;
  initialDelayMs: number;
  maxDelayMs: number;
  retryableErrors: RetryableErrorCode[];
}

export interface TranslationAttemptMetrics {
  provider: ProviderId;
  modelId: string;
  durationMs: number;
  inputTokens?: number;
  outputTokens?: number;
  retries: number;
  success: boolean;
  errorCode?: RetryableErrorCode | null;
}

export interface TranslationFileDescriptor {
  relativePath: string;
  modInstallPath: string;
  /** 아카이브 내부 파일인 경우 아카이브의 상대 경로 */
  archivePath?: string;
  /** 아카이브 내부 엔트리 경로 */
  archiveEntryPath?: string;
}

export interface StartTranslationJobPayload {
  jobId: string;
  provider: ProviderId;
  apiKey: string | null;
  modelId: string;
  files: TranslationFileDescriptor[];
  sourceLang: string | null;
  targetLang: string | null;
  outputOverrideDir?: string | null;
  resumeFromCheckpoint?: boolean;
  resetResumeState?: boolean;
}

export type TranslationProgressState = JobState;

export interface TranslationFileErrorEntry {
  filePath: string;
  message: string;
  code?: RetryableErrorCode | string;
}

export interface TranslationRetryInfo {
  attempt: number;
  maxAttempts: number;
  delaySeconds: number;
  reason: string;
}

export interface TranslationResumeHint {
  filePath: string;
  lineNumber: number;
}

export interface TranslationCheckpoint {
  currentFilePath?: string | null;
  nextLineIndex?: number | null;
  translatedCount: number;
  totalCount: number;
}

export interface TranslationProgressEventPayload {
  jobId: string;
  status: TranslationProgressState;
  progressPct?: number;
  cancelRequested?: boolean;
  log?: string | null;
  translatedCount?: number;
  totalCount?: number;
  fileName?: string | null;
  fileSuccess?: boolean | null;
  fileErrors?: TranslationFileErrorEntry[];
  lastWritten?: {
    sourceRelativePath: string;
    outputAbsolutePath: string;
    outputRelativePath: string;
  };
  retry?: TranslationRetryInfo;
  resumeHint?: TranslationResumeHint;
  checkpoint?: TranslationCheckpoint | null;
  metrics?: TranslationAttemptMetrics;
}

export interface TranslationBackoffStartedPayload {
  jobId: string;
  delayMs: number;
  attempt: number;
  maxAttempts: number;
  reason: string;
  usedHint: boolean;
}

export interface TranslationBackoffCancelledPayload {
  jobId: string;
  by: "user" | "logic";
}

export interface TranslationRetryStartedPayload {
  jobId: string;
  attempt: number;
}

export type ArchiveType = "jar" | "zip";

export interface ModFileDescriptor {
  path: string;
  mod_install_path: string;
  translatable: boolean;
  auto_selected: boolean;
  language_hint: string | null;
  /** 아카이브 내부 파일인 경우 아카이브의 상대 경로 */
  archive_path?: string;
  /** 아카이브 타입 */
  archive_type?: ArchiveType;
}

export interface ModFileListing {
  files: ModFileDescriptor[];
}

// Placeholder validator types
export type ValidationErrorCode =
  | "PLACEHOLDER_MISMATCH"
  | "PAIR_UNBALANCED"
  | "FORMAT_TOKEN_MISSING"
  | "XML_MALFORMED_AFTER_RESTORE"
  | "RETRY_FAILED";

export type RecoveryStep =
  | "REINJECT_MISSING_PROTECTED"
  | "PAIR_BALANCE_CHECK"
  | "REMOVE_EXCESS_TOKENS"
  | "CORRECT_FORMAT_TOKENS"
  | "PRESERVE_PERCENT_BINDING";

export interface AutofixResult {
  applied: boolean;
  steps: RecoveryStep[];
}

export interface RetryInfo {
  attempted: boolean;
  success?: boolean;
}

export interface UiHint {
  showSource: boolean;
  showCandidate: boolean;
  copyButtons: boolean;
}

export interface ValidationFailureReport {
  code: ValidationErrorCode;
  file: string;
  line: number;
  key: string;
  expectedProtected: string[];
  foundProtected: string[];
  expectedFormat: string[];
  foundFormat: string[];
  expectedStructureSignature: string[];
  foundStructureSignature: string[];
  sourceLine: string;
  preprocessedSource: string;
  candidateLine: string;
  autofix: AutofixResult;
  retry: RetryInfo;
  uiHint: UiHint;
}

export interface ValidatorConfig {
  enableAutofix: boolean;
  retryOnFail: boolean;
  retryLimit: number;
  strictPairing: boolean;
  preservePercentBinding: boolean;
}

export interface ValidationMetrics {
  totalValidations: number;
  totalFailures: number;
  autofixAttempts: number;
  autofixSuccesses: number;
  retryAttempts: number;
  retrySuccesses: number;
  byErrorCode: Record<string, number>;
}

// Legacy pipeline-related types removed in favor of streaming progress events.
