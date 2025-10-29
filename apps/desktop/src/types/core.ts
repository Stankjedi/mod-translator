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

export type JobState = 'pending' | 'running' | 'completed' | 'failed' | 'canceled'

export type ProviderId = 'gemini' | 'gpt' | 'claude' | 'grok'

export interface TranslationFileDescriptor {
  path: string
}

export interface StartTranslationJobPayload {
  jobId: string
  provider: ProviderId
  apiKey: string | null
  files: TranslationFileDescriptor[]
  sourceLang: string | null
  targetLang: string | null
}

export type TranslationProgressState = JobState

export interface TranslationProgressEventPayload {
  jobId: string
  state: TranslationProgressState
  progress: number
  log?: string | null
  translatedCount: number
  totalCount: number
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

// Legacy pipeline-related types removed in favor of streaming progress events.
