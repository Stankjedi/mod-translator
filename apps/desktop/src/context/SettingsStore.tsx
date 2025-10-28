/* eslint-disable react-refresh/only-export-components */
import type { ReactNode } from 'react'
import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'
import {
  DEFAULT_PERSISTED_SETTINGS,
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

interface SettingsStoreValue extends SettingsState {
  setProviderEnabled: (provider: ProviderId, enabled: boolean) => void
  toggleProvider: (provider: ProviderId) => void
  setActiveProvider: (provider: ProviderId) => void
  updateApiKey: (provider: ProviderId, value: string | null) => void
  setConcurrency: (value: number) => void
  setWorkerCount: (value: number) => void
  setBucketSize: (value: number) => void
  setRefillMs: (value: number) => void
  setEnableBackendLogging: (enabled: boolean) => void
  setEnforcePlaceholderGuard: (enabled: boolean) => void
  setPrioritizeDllResources: (enabled: boolean) => void
  setEnableQualitySampling: (enabled: boolean) => void
}

const SettingsStoreContext = createContext<SettingsStoreValue | undefined>(undefined)

const PROVIDER_ORDER: ProviderId[] = ['gemini', 'gpt', 'claude', 'grok']

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

  useEffect(() => {
    try {
      persistSettings({
        selectedProviders: state.selectedProviders,
        activeProviderId: state.activeProviderId,
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

  const updateApiKey = useCallback((provider: ProviderId, value: string | null) => {
    let outcome: { success: boolean; error?: unknown } = { success: true }
    setState((prev) => {
      const nextKeys: ApiKeyMap = { ...prev.apiKeys }
      const trimmed = (value ?? '').trim()
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
  }, [])

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
      setProviderEnabled,
      toggleProvider,
      setActiveProvider,
      updateApiKey,
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
      updateApiKey,
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
