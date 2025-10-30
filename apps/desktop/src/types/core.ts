export type LibraryStatus = 'healthy' | 'missing'

export interface FormattedTimestamp {
  iso_date: string
}

export interface PolicyBanner {
  headline: string
  message: string
  requires_acknowledgement: boolean
  checkbox_label: string
  warning: string
}

export interface PolicyProfile {
  game: string
  redistribution_blocked: boolean
  requires_author_permission: boolean
  eula_reference: string | null
  notes: string[]
}

export interface ModSummary {
  id: string
  name: string
  game: string
  directory: string
  installed_languages: string[]
  last_updated: FormattedTimestamp
  policy: PolicyProfile
  warnings: string[]
}

export interface LibraryEntry {
  path: string
  status: LibraryStatus
  mods: ModSummary[]
  workshop_root: string | null
  notes: string[]
}

export interface LibraryScanResponse {
  libraries: LibraryEntry[]
  policy_banner: PolicyBanner
}

export interface SteamPathResponse {
  path: string | null
  note: string
}

export type JobState =
  | 'pending'
  | 'running'
  | 'completed'
  | 'failed'
  | 'canceled'
  | 'partial_success'

export type ProviderId = 'gemini' | 'gpt' | 'claude' | 'grok'

export type RetryableErrorCode =
  | 'RATE_LIMITED'
  | 'NETWORK_TRANSIENT'
  | 'SERVER_TRANSIENT'
  | 'NETWORK'
  | 'TIMEOUT'
  | 'SERVER_5XX'
  | 'BAD_GATEWAY'
  | 'SERVICE_UNAVAILABLE'

export interface RetryPolicy {
  maxAttempts: number
  initialDelayMs: number
  maxDelayMs: number
  retryableErrors: RetryableErrorCode[]
}

export interface TranslationAttemptMetrics {
  provider: ProviderId
  modelId: string
  durationMs: number
  inputTokens?: number
  outputTokens?: number
  retries: number
  success: boolean
  errorCode?: RetryableErrorCode | null
}

export interface TranslationFileDescriptor {
  relativePath: string
  modInstallPath: string
}

export interface StartTranslationJobPayload {
  jobId: string
  provider: ProviderId
  apiKey: string | null
  modelId: string
  files: TranslationFileDescriptor[]
  sourceLang: string | null
  targetLang: string | null
  outputOverrideDir?: string | null
  resumeFromCheckpoint?: boolean
  resetResumeState?: boolean
}

export type TranslationProgressState = JobState

export interface TranslationFileErrorEntry {
  filePath: string
  message: string
  code?: RetryableErrorCode | string
}

export interface TranslationRetryInfo {
  attempt: number
  maxAttempts: number
  delaySeconds: number
  reason: string
}

export interface TranslationResumeHint {
  filePath: string
  lineNumber: number
}

export interface TranslationCheckpoint {
  currentFilePath?: string | null
  nextLineIndex?: number | null
  translatedCount: number
  totalCount: number
}

export interface TranslationProgressEventPayload {
  jobId: string
  status: TranslationProgressState
  progressPct?: number
  cancelRequested?: boolean
  log?: string | null
  translatedCount?: number
  totalCount?: number
  fileName?: string | null
  fileSuccess?: boolean | null
  fileErrors?: TranslationFileErrorEntry[]
  lastWritten?: {
    sourceRelativePath: string
    outputAbsolutePath: string
    outputRelativePath: string
  }
  retry?: TranslationRetryInfo
  resumeHint?: TranslationResumeHint
  checkpoint?: TranslationCheckpoint | null
  metrics?: TranslationAttemptMetrics
}

export interface ModFileDescriptor {
  path: string
  mod_install_path: string
  translatable: boolean
  auto_selected: boolean
  language_hint: string | null
}

export interface ModFileListing {
  files: ModFileDescriptor[]
}

// Legacy pipeline-related types removed in favor of streaming progress events.
