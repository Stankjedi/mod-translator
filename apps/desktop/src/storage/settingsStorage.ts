const STORAGE_KEY = 'mod_translator_settings_v1'

export type ProviderId = 'gemini' | 'gpt' | 'claude' | 'grok'

export type ProviderModelMap = Record<ProviderId, string>
export type ProviderModelListMap = Record<ProviderId, string[]>

export const PROVIDER_MODEL_OPTIONS: Record<ProviderId, string[]> = {
  gemini: ['gemini-2.5-flash', 'gemini-2.5-pro', 'gemini-2.5-flash-lite'],
  gpt: ['gpt-4o-mini', 'gpt-4o'],
  claude: ['claude-3-5-sonnet-20240620', 'claude-3-opus-20240229', 'claude-3-haiku-20240307'],
  grok: ['grok-2-1212'],
}

export const DEFAULT_PROVIDER_MODELS: ProviderModelMap = {
  gemini: 'gemini-2.5-flash',
  gpt: 'gpt-4o-mini',
  claude: 'claude-3-5-sonnet-20240620',
  grok: 'grok-2-1212',
}

const DEFAULT_VERIFIED_MODELS: ProviderModelListMap = {
  gemini: [],
  gpt: [],
  claude: [],
  grok: [],
}

export interface PersistedSettings {
  selectedProviders: ProviderId[]
  activeProviderId: ProviderId
  providerModels: ProviderModelMap
  verifiedModels: ProviderModelListMap
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
  selectedProviders: ['gemini', 'gpt', 'claude', 'grok'],
  activeProviderId: 'gemini',
  providerModels: { ...DEFAULT_PROVIDER_MODELS },
  verifiedModels: { ...DEFAULT_VERIFIED_MODELS },
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

function sanitizeProviderModels(value: unknown): ProviderModelMap {
  const defaults = { ...DEFAULT_PROVIDER_MODELS }
  if (!value || typeof value !== 'object') {
    return defaults
  }

  const entries = value as Record<string, unknown>
  (Object.keys(defaults) as ProviderId[]).forEach((provider) => {
    const raw = entries[provider]
    const normalized = typeof raw === 'string' ? raw.trim() : ''
    if (normalized) {
      defaults[provider] = normalized
    }
  })

  return defaults
}

function sanitizeVerifiedModels(value: unknown): ProviderModelListMap {
  const defaults = { ...DEFAULT_VERIFIED_MODELS }
  if (!value || typeof value !== 'object') {
    return defaults
  }

  const entries = value as Record<string, unknown>
  ;(Object.keys(defaults) as ProviderId[]).forEach((provider) => {
    const raw = entries[provider]
    if (!Array.isArray(raw)) {
      return
    }
    const normalized = Array.from(
      new Set(
        raw
          .map((item) => (typeof item === 'string' ? item.trim() : ''))
          .filter((item): item is string => Boolean(item)),
      ),
    ).sort((a, b) => a.localeCompare(b))
    defaults[provider] = normalized
  })

  return defaults
}

function sanitizeActiveProvider(
  value: unknown,
  selectedProviders: ProviderId[],
): ProviderId {
  if (typeof value === 'string') {
    const normalized = value.toLowerCase()
    if (selectedProviders.includes(normalized as ProviderId)) {
      return normalized as ProviderId
    }
  }

  return selectedProviders[0] ?? DEFAULT_PERSISTED_SETTINGS.activeProviderId
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

    const selectedProviders = sanitizeProviders(parsed.selectedProviders)
    const providerModels = sanitizeProviderModels(parsed.providerModels)
    const verifiedModels = sanitizeVerifiedModels(parsed.verifiedModels)

    return {
      selectedProviders,
      activeProviderId: sanitizeActiveProvider(
        parsed.activeProviderId,
        selectedProviders,
      ),
      providerModels,
      verifiedModels,
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

  const sanitizedProviders = sanitizeProviders(settings.selectedProviders)
  const activeProviderId = sanitizeActiveProvider(
    settings.activeProviderId,
    sanitizedProviders,
  )
  const providerModels = sanitizeProviderModels(settings.providerModels)
  const verifiedModels = sanitizeVerifiedModels(settings.verifiedModels)

  ;(Object.keys(providerModels) as ProviderId[]).forEach((provider) => {
    const verifiedList = verifiedModels[provider] ?? []
    const current = providerModels[provider]
    if (verifiedList.length === 0) {
      providerModels[provider] = DEFAULT_PROVIDER_MODELS[provider]
      return
    }
    if (!verifiedList.includes(current)) {
      providerModels[provider] = verifiedList[0] ?? DEFAULT_PROVIDER_MODELS[provider]
    }
  })

  const payload: PersistedSettings = {
    ...DEFAULT_PERSISTED_SETTINGS,
    ...settings,
    selectedProviders: sanitizedProviders,
    activeProviderId,
    providerModels,
    verifiedModels,
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
