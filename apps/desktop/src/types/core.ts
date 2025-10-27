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

export type JobState = 'queued' | 'running' | 'completed' | 'failed'

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
}
