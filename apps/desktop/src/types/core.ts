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

export interface ProviderAuth {
  gemini?: string | null
  gpt?: string | null
  claude?: string | null
  grok?: string | null
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

export type JobState = 'queued' | 'running' | 'completed' | 'failed' | 'canceled'

export type TranslatorKind = 'gemini' | 'gpt' | 'claude' | 'grok'

export interface TranslationJobRequest {
  mod_id: string
  mod_name: string
  translator: TranslatorKind
  source_language_guess: string | null
  target_language: string
  selected_files: string[]
  provider_auth: ProviderAuth
}

export interface ModFileDescriptor {
  path: string
  translatable: boolean
  auto_selected: boolean
  language_hint: string | null
}

export interface ModFileListing {
  files: ModFileDescriptor[]
}

export interface QueueSnapshot {
  queued: number
  running: number
  concurrent_workers: number
}

export interface RateLimiterSnapshot {
  bucket_capacity: number
  tokens_available: number
  refill_interval_ms: number
}

export interface QualityGateSnapshot {
  placeholder_guard: boolean
  format_validator: boolean
  sample_rate: number
}

export type StageStrategy =
  | 'enumerate'
  | 'detect'
  | 'parse'
  | 'translate'
  | 'validate'
  | 'repackage'

export interface PipelineStage {
  name: string
  description: string
  strategy: StageStrategy
}

export interface ValidatorSpec {
  name: string
  description: string
}

export interface PipelinePlan {
  target: string
  stages: PipelineStage[]
  validators: ValidatorSpec[]
  skip_rules: string[]
}

export interface TranslationJobStatus {
  job_id: string
  translator: string
  state: JobState
  progress: number
  preview: string | null
  message: string | null
  queue: QueueSnapshot
  rate_limiter: RateLimiterSnapshot
  quality_gates: QualityGateSnapshot
  pipeline: PipelinePlan
  cancel_requested: boolean
}

export interface JobStatusUpdatedEvent {
  job_id: string
  mod_id: string
  status: TranslationJobStatus
}
