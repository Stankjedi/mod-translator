const STORAGE_KEY = 'mod_translator_settings_v1'

export type ProviderId = 'gemini' | 'gpt' | 'claude' | 'grok'

export interface PersistedSettings {
  selectedProviders: ProviderId[]
  concurrency: number
  workerCount: number
  bucketSize: number
  refillMs: number
  enableBackendLogging: boolean
  enforcePlaceholderGuard: boolean
  prioritizeDllResources: boolean
  enableQualitySampling: boolean
}

export const DEFAULT_PERSISTED_SETTINGS: PersistedSettings = {
  selectedProviders: ['gemini', 'gpt'],
  concurrency: 3,
  workerCount: 2,
  bucketSize: 5,
  refillMs: 750,
  enableBackendLogging: false,
  enforcePlaceholderGuard: true,
  prioritizeDllResources: true,
  enableQualitySampling: true,
}

function isStorageAvailable() {
  return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined'
}

function sanitizeProviders(input: unknown): ProviderId[] {
  if (!Array.isArray(input)) {
    return [...DEFAULT_PERSISTED_SETTINGS.selectedProviders]
  }

  const seen = new Set<ProviderId>()
  const order: ProviderId[] = ['gemini', 'gpt', 'claude', 'grok']

  input.forEach((value) => {
    if (typeof value !== 'string') {
      return
    }
    if (order.includes(value as ProviderId)) {
      seen.add(value as ProviderId)
    }
  })

  const normalized = order.filter((provider) => seen.has(provider))
  return normalized.length ? normalized : [...DEFAULT_PERSISTED_SETTINGS.selectedProviders]
}

function sanitizeNumber(value: unknown, fallback: number, min: number) {
  const parsed = Number(value)
  if (!Number.isFinite(parsed)) {
    return fallback
  }
  return Math.max(min, Math.round(parsed))
}

function sanitizeBoolean(value: unknown, fallback: boolean) {
  return typeof value === 'boolean' ? value : fallback
}

export function loadPersistedSettings(): PersistedSettings {
  if (!isStorageAvailable()) {
    return { ...DEFAULT_PERSISTED_SETTINGS }
  }

  const raw = window.localStorage.getItem(STORAGE_KEY)
  if (!raw) {
    return { ...DEFAULT_PERSISTED_SETTINGS }
  }

  try {
    const parsed = JSON.parse(raw) as Partial<PersistedSettings>

    return {
      selectedProviders: sanitizeProviders(parsed.selectedProviders),
      concurrency: sanitizeNumber(parsed.concurrency, DEFAULT_PERSISTED_SETTINGS.concurrency, 1),
      workerCount: sanitizeNumber(parsed.workerCount, DEFAULT_PERSISTED_SETTINGS.workerCount, 1),
      bucketSize: sanitizeNumber(parsed.bucketSize, DEFAULT_PERSISTED_SETTINGS.bucketSize, 1),
      refillMs: sanitizeNumber(parsed.refillMs, DEFAULT_PERSISTED_SETTINGS.refillMs, 50),
      enableBackendLogging: sanitizeBoolean(
        parsed.enableBackendLogging,
        DEFAULT_PERSISTED_SETTINGS.enableBackendLogging,
      ),
      enforcePlaceholderGuard: sanitizeBoolean(
        parsed.enforcePlaceholderGuard,
        DEFAULT_PERSISTED_SETTINGS.enforcePlaceholderGuard,
      ),
      prioritizeDllResources: sanitizeBoolean(
        parsed.prioritizeDllResources,
        DEFAULT_PERSISTED_SETTINGS.prioritizeDllResources,
      ),
      enableQualitySampling: sanitizeBoolean(
        parsed.enableQualitySampling,
        DEFAULT_PERSISTED_SETTINGS.enableQualitySampling,
      ),
    }
  } catch (error) {
    console.error('Failed to parse persisted settings, using defaults.', error)
    return { ...DEFAULT_PERSISTED_SETTINGS }
  }
}

export function persistSettings(settings: PersistedSettings) {
  if (!isStorageAvailable()) {
    throw new Error('localStorage is not available')
  }

  const payload: PersistedSettings = {
    ...DEFAULT_PERSISTED_SETTINGS,
    ...settings,
    selectedProviders: sanitizeProviders(settings.selectedProviders),
    concurrency: sanitizeNumber(settings.concurrency, DEFAULT_PERSISTED_SETTINGS.concurrency, 1),
    workerCount: sanitizeNumber(settings.workerCount, DEFAULT_PERSISTED_SETTINGS.workerCount, 1),
    bucketSize: sanitizeNumber(settings.bucketSize, DEFAULT_PERSISTED_SETTINGS.bucketSize, 1),
    refillMs: sanitizeNumber(settings.refillMs, DEFAULT_PERSISTED_SETTINGS.refillMs, 50),
    enableBackendLogging: Boolean(settings.enableBackendLogging),
    enforcePlaceholderGuard: Boolean(settings.enforcePlaceholderGuard),
    prioritizeDllResources: Boolean(settings.prioritizeDllResources),
    enableQualitySampling: Boolean(settings.enableQualitySampling),
  }

  window.localStorage.setItem(STORAGE_KEY, JSON.stringify(payload))
}
