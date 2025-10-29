/* eslint-disable react-refresh/only-export-components */
import type { ReactNode } from 'react'
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react'
import { invoke } from '@tauri-apps/api/core'
import {
  DEFAULT_PERSISTED_SETTINGS,
  PROVIDER_MODEL_OPTIONS,
  loadPersistedSettings,
  persistSettings,
  type PersistedSettings,
  type ProviderId,
} from '../storage/settingsStorage'
import { loadApiKeys, persistApiKeys, type ApiKeyMap } from '../storage/apiKeyStorage'

export type { ProviderId }

interface SettingsState extends PersistedSettings {
  apiKeys: ApiKeyMap
}

export type KeyValidationState = 'valid' | 'unauthorized' | 'forbidden' | 'network_error'

interface SettingsStoreValue extends SettingsState {
  providerModelOptions: Record<ProviderId, string[]>
  keyValidation: Record<ProviderId, KeyValidationState | null>
  validationInFlight: Record<ProviderId, boolean>
  modelDiscoveryState: Record<ProviderId, ProviderModelDiscoveryState>
  providerModelNotices: Record<ProviderId, string | null>
  setProviderEnabled: (provider: ProviderId, enabled: boolean) => void
  toggleProvider: (provider: ProviderId) => void
  setActiveProvider: (provider: ProviderId) => void
  setProviderModel: (provider: ProviderId, modelId: string) => void
  updateApiKey: (provider: ProviderId, value: string | null) => void
  refreshProviderModels: (provider: ProviderId, apiKeyOverride?: string) => Promise<void>
  revalidateProviderKey: (
    provider: ProviderId,
    apiKeyOverride?: string,
    modelOverride?: string,
  ) => Promise<KeyValidationState>
  setConcurrency: (value: number) => void
  setWorkerCount: (value: number) => void
  setBucketSize: (value: number) => void
  setRefillMs: (value: number) => void
  setEnableBackendLogging: (enabled: boolean) => void
  setEnforcePlaceholderGuard: (enabled: boolean) => void
  setPrioritizeDllResources: (enabled: boolean) => void
  setEnableQualitySampling: (enabled: boolean) => void
}

type ProviderModelSource = 'live' | 'fallback'

interface ProviderModelDiscoveryState {
  source: ProviderModelSource
  networkError: boolean
}

interface ProviderValidationResponse {
  validationStatus: KeyValidationState
  models: string[]
}

function createValidationResponse(
  status: KeyValidationState,
  models: string[] = [],
): ProviderValidationResponse {
  return {
    validationStatus: status,
    models: models.length > 0 ? [...models] : [],
  }
}

const SettingsStoreContext = createContext<SettingsStoreValue | undefined>(undefined)

const PROVIDER_ORDER: ProviderId[] = ['gemini', 'gpt', 'claude', 'grok']

function normalizeModelList(models: string[]): string[] {
  const normalized = Array.from(
    new Set(
      models
        .map((model) => (typeof model === 'string' ? model.trim() : ''))
        .filter((model): model is string => Boolean(model)),
    ),
  )
  normalized.sort((a, b) => a.localeCompare(b))
  return normalized
}

function mergeModelOptions(primary: string[] | undefined, fallback: string[]): string[] {
  const normalizedPrimary = normalizeModelList(primary ?? [])
  const normalizedFallback = normalizeModelList(fallback)
  const extras = normalizedFallback.filter((model) => !normalizedPrimary.includes(model))
  return [...normalizedPrimary, ...extras]
}

function arraysEqual(a: string[], b: string[]) {
  if (a.length !== b.length) return false
  return a.every((value, index) => value === b[index])
}

const clampPositiveInteger = (value: number, minimum: number) => {
  if (!Number.isFinite(value)) return minimum
  return Math.max(minimum, Math.round(value))
}

export function SettingsStoreProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<SettingsState>(() => ({
    ...DEFAULT_PERSISTED_SETTINGS,
    ...loadPersistedSettings(),
    apiKeys: loadApiKeys(),
  }))
  const [providerModelOptions, setProviderModelOptions] = useState<Record<ProviderId, string[]>>({
    gemini: mergeModelOptions(state.verifiedModels.gemini, PROVIDER_MODEL_OPTIONS.gemini),
    gpt: mergeModelOptions(state.verifiedModels.gpt, PROVIDER_MODEL_OPTIONS.gpt),
    claude: mergeModelOptions(state.verifiedModels.claude, PROVIDER_MODEL_OPTIONS.claude),
    grok: mergeModelOptions(state.verifiedModels.grok, PROVIDER_MODEL_OPTIONS.grok),
  })
  const [keyValidation, setKeyValidation] = useState<Record<ProviderId, KeyValidationState | null>>({
    gemini: null,
    gpt: null,
    claude: null,
    grok: null,
  })
  const [validationInFlight, setValidationInFlight] = useState<Record<ProviderId, boolean>>({
    gemini: false,
    gpt: false,
    claude: false,
    grok: false,
  })
  const [modelDiscoveryState, setModelDiscoveryState] = useState<
    Record<ProviderId, ProviderModelDiscoveryState>
  >({
    gemini: { source: 'fallback', networkError: false },
    gpt: { source: 'fallback', networkError: false },
    claude: { source: 'fallback', networkError: false },
    grok: { source: 'fallback', networkError: false },
  })
  const [providerModelNotices, setProviderModelNotices] = useState<Record<ProviderId, string | null>>({
    gemini: null,
    gpt: null,
    claude: null,
    grok: null,
  })
  const validationLocks = useRef<
    Record<
      ProviderId,
      { promise: Promise<ProviderValidationResponse>; key: string; model: string } | null
    >
  >({
    gemini: null,
    gpt: null,
    claude: null,
    grok: null,
  })

  const runValidation = useCallback(
    async (
      provider: ProviderId,
      apiKeyOverride?: string,
      modelOverride?: string,
    ): Promise<ProviderValidationResponse> => {
      const trimmed = (apiKeyOverride ?? state.apiKeys[provider] ?? '').trim()
      const modelHint = (modelOverride ?? state.providerModels[provider] ?? '').trim()

      const lock = validationLocks.current[provider]
      if (lock) {
        if (lock.key === trimmed && lock.model === modelHint) {
          return lock.promise
        }
        await lock.promise
      }

      if (!trimmed) {
        setValidationInFlight((prev) => ({
          ...prev,
          [provider]: false,
        }))
        return createValidationResponse('unauthorized')
      }

      setValidationInFlight((prev) => ({
        ...prev,
        [provider]: true,
      }))

      const task = (async () => {
        try {
          const response = await invoke<ProviderValidationResponse>('validate_api_key_and_list_models', {
            provider,
            apiKey: trimmed,
            modelHint: modelHint || undefined,
          })
          return createValidationResponse(response.validationStatus, response.models)
        } catch (error) {
          console.error(`Failed to validate API key for ${provider}`, error)
          return createValidationResponse('network_error')
        }
      })()

      validationLocks.current[provider] = { promise: task, key: trimmed, model: modelHint }

      const result = await task

      validationLocks.current[provider] = null
      setValidationInFlight((prev) => ({
        ...prev,
        [provider]: false,
      }))

      return result
    },
    [setValidationInFlight, state.apiKeys, state.providerModels],
  )

  useEffect(() => {
    setProviderModelOptions((prev) => {
      const next: Record<ProviderId, string[]> = {
        gemini: mergeModelOptions(state.verifiedModels.gemini, PROVIDER_MODEL_OPTIONS.gemini),
        gpt: mergeModelOptions(state.verifiedModels.gpt, PROVIDER_MODEL_OPTIONS.gpt),
        claude: mergeModelOptions(state.verifiedModels.claude, PROVIDER_MODEL_OPTIONS.claude),
        grok: mergeModelOptions(state.verifiedModels.grok, PROVIDER_MODEL_OPTIONS.grok),
      }

      const changed = (Object.keys(next) as ProviderId[]).some((provider) => {
        const previous = prev[provider] ?? []
        const current = next[provider]
        return !arraysEqual(previous, current)
      })

      return changed ? next : prev
    })
  }, [state.verifiedModels])

  useEffect(() => {
    setModelDiscoveryState((prev) => {
      let changed = false
      const next: Record<ProviderId, ProviderModelDiscoveryState> = { ...prev }

      ;(Object.keys(state.verifiedModels) as ProviderId[]).forEach((provider) => {
        const hasVerified = (state.verifiedModels[provider] ?? []).length > 0
        if (!hasVerified) {
          return
        }
        const current = prev[provider]
        if (!current || current.source !== 'live' || current.networkError) {
          next[provider] = { source: 'live', networkError: false }
          changed = true
        }
      })

      return changed ? next : prev
    })
  }, [state.verifiedModels])

  const applyValidationResult = useCallback(
    (provider: ProviderId, response: ProviderValidationResponse) => {
      const fallback = [...PROVIDER_MODEL_OPTIONS[provider]]
      const normalized = normalizeModelList(response.models)
      const status = response.validationStatus

      setKeyValidation((prev) => ({
        ...prev,
        [provider]: status,
      }))

      let mergedVerified: string[] = []
      let combinedOptions: string[] = mergeModelOptions([], fallback)
      let nextModelValue: string | null = null
      let forbiddenModel: string | null = null

      setState((prev) => {
        const currentVerified = prev.verifiedModels[provider] ?? []
        const currentModel = prev.providerModels[provider] ?? ''

        let nextVerified = currentVerified
        let nextModel = currentModel

        if (status === 'valid' && normalized.length > 0) {
          const merged = normalizeModelList([...currentVerified, ...normalized])
          nextVerified = merged
        }

        const allowedOptions = mergeModelOptions(nextVerified, fallback)
        if (status === 'forbidden' && currentModel && !nextVerified.includes(currentModel)) {
          forbiddenModel = currentModel
        }

        if (status === 'valid' && normalized.length > 0 && !nextVerified.includes(nextModel)) {
          nextModel = allowedOptions[0] ?? ''
        }

        if (!allowedOptions.includes(nextModel)) {
          nextModel = allowedOptions[0] ?? ''
        }

        const verifiedChanged = !arraysEqual(nextVerified, currentVerified)
        const modelChanged = nextModel !== currentModel

        mergedVerified = nextVerified
        combinedOptions = allowedOptions
        nextModelValue = nextModel

        if (!verifiedChanged && !modelChanged) {
          return prev
        }

        return {
          ...prev,
          providerModels: modelChanged
            ? {
                ...prev.providerModels,
                [provider]: nextModel,
              }
            : prev.providerModels,
          verifiedModels: verifiedChanged
            ? {
                ...prev.verifiedModels,
                [provider]: nextVerified,
              }
            : prev.verifiedModels,
        }
      })

      setProviderModelOptions((prev) => ({
        ...prev,
        [provider]: combinedOptions,
      }))

      setModelDiscoveryState((prev) => ({
        ...prev,
        [provider]: {
          source: mergedVerified.length > 0 ? 'live' : 'fallback',
          networkError: status === 'network_error',
        },
      }))

      setProviderModelNotices((prev) => {
        if (status === 'valid') {
          if (prev[provider] === null) {
            return prev
          }
          return {
            ...prev,
            [provider]: null,
          }
        }

        if (status === 'forbidden') {
          const modelName = forbiddenModel ?? nextModelValue ?? ''
          const notice = modelName
            ? `${modelName} 모델은 현재 API 키로 사용할 수 없습니다. 다른 모델을 선택해 주세요.`
            : '선택한 모델을 사용할 수 없습니다. 다른 모델을 선택해 주세요.'
          return {
            ...prev,
            [provider]: notice,
          }
        }

        if (prev[provider] === null) {
          return prev
        }

        if (status === 'unauthorized' || status === 'network_error') {
          return {
            ...prev,
            [provider]: null,
          }
        }

        return prev
      })
    },
    [setModelDiscoveryState, setProviderModelNotices, setProviderModelOptions, setState],
  )

  const revalidateProviderKey = useCallback(
    async (provider: ProviderId, apiKeyOverride?: string, modelOverride?: string) => {
      const result = await runValidation(provider, apiKeyOverride, modelOverride)
      applyValidationResult(provider, result)
      return result.validationStatus
    },
    [applyValidationResult, runValidation],
  )

  const refreshProviderModels = useCallback(
    async (provider: ProviderId, apiKeyOverride?: string) => {
      await revalidateProviderKey(provider, apiKeyOverride)
    },
    [revalidateProviderKey],
  )

  useEffect(() => {
    try {
      persistSettings({
        selectedProviders: state.selectedProviders,
        activeProviderId: state.activeProviderId,
        providerModels: state.providerModels,
        verifiedModels: state.verifiedModels,
        concurrency: state.concurrency,
        workerCount: state.workerCount,
        bucketSize: state.bucketSize,
        refillMs: state.refillMs,
        enableBackendLogging: state.enableBackendLogging,
        enforcePlaceholderGuard: state.enforcePlaceholderGuard,
        prioritizeDllResources: state.prioritizeDllResources,
        enableQualitySampling: state.enableQualitySampling,
      })
    } catch (error) {
      console.error('Failed to persist settings', error)
    }
  }, [
    state.selectedProviders,
    state.activeProviderId,
    state.providerModels,
    state.verifiedModels,
    state.concurrency,
    state.workerCount,
    state.bucketSize,
    state.refillMs,
    state.enableBackendLogging,
    state.enforcePlaceholderGuard,
    state.prioritizeDllResources,
    state.enableQualitySampling,
  ])

  const setProviderEnabled = useCallback((provider: ProviderId, enabled: boolean) => {
    setState((prev) => {
      const set = new Set(prev.selectedProviders)
      if (enabled) {
        set.add(provider)
      } else {
        set.delete(provider)
      }

      const ordered = PROVIDER_ORDER.filter((item) => set.has(item))
      const nextProviders = ordered.length
        ? ordered
        : [...DEFAULT_PERSISTED_SETTINGS.selectedProviders]
      const nextActive = nextProviders.includes(prev.activeProviderId)
        ? prev.activeProviderId
        : nextProviders[0]
      return {
        ...prev,
        selectedProviders: nextProviders,
        activeProviderId: nextActive,
      }
    })
  }, [])

  const toggleProvider = useCallback(
    (provider: ProviderId) => {
      setState((prev) => {
        const set = new Set(prev.selectedProviders)
        if (set.has(provider)) {
          set.delete(provider)
        } else {
          set.add(provider)
        }
        const ordered = PROVIDER_ORDER.filter((item) => set.has(item))
        const nextProviders = ordered.length
          ? ordered
          : [...DEFAULT_PERSISTED_SETTINGS.selectedProviders]
        const nextActive = nextProviders.includes(prev.activeProviderId)
          ? prev.activeProviderId
          : nextProviders[0]
        return {
          ...prev,
          selectedProviders: nextProviders,
          activeProviderId: nextActive,
        }
      })
    },
    [],
  )

  const setActiveProvider = useCallback((provider: ProviderId) => {
    setState((prev) => {
      if (!prev.selectedProviders.includes(provider) || prev.activeProviderId === provider) {
        return prev
      }

      return {
        ...prev,
        activeProviderId: provider,
      }
    })
  }, [])

  const setProviderModel = useCallback(
    (provider: ProviderId, modelId: string) => {
      const trimmed = modelId.trim()
      if (!trimmed) {
        return
      }

      let changed = false
      setState((prev) => {
        if (prev.providerModels[provider] === trimmed) {
          return prev
        }
        changed = true
        return {
          ...prev,
          providerModels: {
            ...prev.providerModels,
            [provider]: trimmed,
          },
        }
      })

      if (changed) {
        setProviderModelNotices((prev) => ({
          ...prev,
          [provider]: null,
        }))
      }
    },
    [setProviderModelNotices],
  )

  const updateApiKey = useCallback(
    (provider: ProviderId, value: string | null) => {
      let outcome: { success: boolean; error?: unknown } = { success: true }
      let trimmedValue = ''
      setState((prev) => {
        const nextKeys: ApiKeyMap = { ...prev.apiKeys }
        const trimmed = (value ?? '').trim()
        trimmedValue = trimmed
        if (trimmed) {
          nextKeys[provider] = trimmed
        } else {
          delete nextKeys[provider]
        }

        try {
          persistApiKeys(nextKeys)
          outcome = { success: true }
          return { ...prev, apiKeys: nextKeys }
        } catch (error) {
          outcome = { success: false, error }
          return prev
        }
      })

      if (!outcome.success) {
        throw outcome.error instanceof Error ? outcome.error : new Error(String(outcome.error))
      }
      setProviderModelNotices((prev) => ({
        ...prev,
        [provider]: null,
      }))
      ;(async () => {
        await revalidateProviderKey(provider, trimmedValue)
      })()
    },
    [revalidateProviderKey, setProviderModelNotices],
  )

  const setConcurrency = useCallback((value: number) => {
    setState((prev) => ({ ...prev, concurrency: clampPositiveInteger(value, 1) }))
  }, [])

  const setWorkerCount = useCallback((value: number) => {
    setState((prev) => ({ ...prev, workerCount: clampPositiveInteger(value, 1) }))
  }, [])

  const setBucketSize = useCallback((value: number) => {
    setState((prev) => ({ ...prev, bucketSize: clampPositiveInteger(value, 1) }))
  }, [])

  const setRefillMs = useCallback((value: number) => {
    setState((prev) => ({ ...prev, refillMs: clampPositiveInteger(value, 50) }))
  }, [])

  const setEnableBackendLogging = useCallback((enabled: boolean) => {
    setState((prev) => ({ ...prev, enableBackendLogging: enabled }))
  }, [])

  const setEnforcePlaceholderGuard = useCallback((enabled: boolean) => {
    setState((prev) => ({ ...prev, enforcePlaceholderGuard: enabled }))
  }, [])

  const setPrioritizeDllResources = useCallback((enabled: boolean) => {
    setState((prev) => ({ ...prev, prioritizeDllResources: enabled }))
  }, [])

  const setEnableQualitySampling = useCallback((enabled: boolean) => {
    setState((prev) => ({ ...prev, enableQualitySampling: enabled }))
  }, [])

  const value = useMemo<SettingsStoreValue>(
    () => ({
      ...state,
      providerModelOptions,
      keyValidation,
      validationInFlight,
      modelDiscoveryState,
      providerModelNotices,
      setProviderEnabled,
      toggleProvider,
      setActiveProvider,
      setProviderModel,
      updateApiKey,
      refreshProviderModels,
      revalidateProviderKey,
      setConcurrency,
      setWorkerCount,
      setBucketSize,
      setRefillMs,
      setEnableBackendLogging,
      setEnforcePlaceholderGuard,
      setPrioritizeDllResources,
      setEnableQualitySampling,
    }),
    [
      state,
      setProviderEnabled,
      toggleProvider,
      setActiveProvider,
      setProviderModel,
      updateApiKey,
      providerModelOptions,
      keyValidation,
      validationInFlight,
      modelDiscoveryState,
      providerModelNotices,
      refreshProviderModels,
      revalidateProviderKey,
      setConcurrency,
      setWorkerCount,
      setBucketSize,
      setRefillMs,
      setEnableBackendLogging,
      setEnforcePlaceholderGuard,
      setPrioritizeDllResources,
      setEnableQualitySampling,
    ],
  )

  return <SettingsStoreContext.Provider value={value}>{children}</SettingsStoreContext.Provider>
}

export function useSettingsStore() {
  const context = useContext(SettingsStoreContext)
  if (!context) {
    throw new Error('SettingsStore must be used within a SettingsStoreProvider')
  }
  return context
}
