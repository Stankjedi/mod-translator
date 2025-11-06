import { useCallback, useEffect, useMemo, useState } from 'react'
import type { ChangeEvent } from 'react'
import { useLibraryContext } from '../context/LibraryContext'
import { useI18n } from '../i18n/ko'
import { maskApiKey } from '../storage/apiKeyStorage'
import {
  useSettingsStore,
  type KeyValidationState,
  type ProviderId,
} from '../context/SettingsStore'
import type { RetryPolicy, RetryableErrorCode } from '../types/core'

const providers: Array<{ id: ProviderId; name: string; description: string }> = [
  {
    id: 'gemini',
    name: '제미니',
    description: 'Google 기반 컨텍스트 확장 모델을 사용합니다.',
  },
  {
    id: 'gpt',
    name: 'GPT',
    description: '긴 컨텍스트와 안정적인 번역 품질을 제공합니다.',
  },
  {
    id: 'claude',
    name: '클로드',
    description: 'Anthropic의 분석 중심 모델로 세밀한 표현에 강합니다.',
  },
  {
    id: 'grok',
    name: '그록',
    description: 'xAI 모델을 통해 빠른 응답과 유연한 문체를 제공합니다.',
  },
]

type RetryPolicyField = 'maxAttempts' | 'initialDelayMs' | 'maxDelayMs'
type RetryNumericField = 'maxRetries' | 'initialDelayMs' | 'multiplier' | 'maxDelayMs'
type RetryBooleanField = 'respectServerRetryAfter' | 'autoTuneConcurrencyOn429'
type ProviderRetryPolicy = import('../storage/settingsStorage').ProviderRetryPolicy

const RETRYABLE_ERROR_CODES: RetryableErrorCode[] = [
  'RATE_LIMITED',
  'NETWORK_TRANSIENT',
  'SERVER_TRANSIENT',
]

const RETRYABLE_ERROR_LABELS: Partial<Record<RetryableErrorCode, string>> = {
  RATE_LIMITED: '429: 요청 제한 (Rate Limit)',
  NETWORK_TRANSIENT: '네트워크/연결 오류',
  SERVER_TRANSIENT: '서버 오류 (5xx)',
}

function statusChip(state: KeyValidationState | null, checking: boolean) {
  if (checking) {
    return {
      label: '확인 중…',
      className: 'bg-sky-700 text-sky-100 border-sky-400/40 animate-pulse',
    }
  }

  switch (state) {
    case 'valid':
      return {
        label: '키 정상',
        className: 'bg-emerald-600 text-emerald-100 border-emerald-400/40',
      }
    case 'unauthorized':
      return {
        label: '인증 실패',
        className: 'bg-rose-600 text-rose-100 border-rose-400/40',
      }
    case 'forbidden':
      return {
        label: '권한 없음',
        className: 'bg-amber-600 text-amber-100 border-amber-400/40',
      }
    case 'network_error':
      return {
        label: '네트워크 오류',
        className: 'bg-slate-600 text-slate-100 border-slate-400/40',
      }
    default:
      return {
        label: '미확인',
        className: 'bg-slate-700 text-slate-200 border-slate-500/40',
      }
  }
}

function formatValidationMessage(
  providerName: string,
  state: KeyValidationState | null,
  verifiedModels: string[],
  selectedModel: string,
  checking: boolean,
) {
  if (checking) {
    return `${providerName} API 키 상태를 확인하는 중입니다. 잠시만 기다려 주세요.`
  }

  if (state === null) {
    return `${providerName} API 키가 아직 확인되지 않았습니다. 키를 저장하거나 "키 확인" 버튼을 눌러 상태를 확인해 주세요.`
  }

  switch (state) {
    case 'valid': {
      if (verifiedModels.length > 0) {
        return `${providerName} API 키가 정상입니다. 사용 가능한 모델: ${verifiedModels.join(', ')}.`
      }
      if (selectedModel) {
        return `${providerName} API 키가 정상입니다. 현재 선택된 모델(${selectedModel})을 사용할 수 있습니다.`
      }
      return `${providerName} API 키가 정상입니다. 확인된 모델이 아직 없습니다. 다른 모델을 선택해 검증해 주세요.`
    }
    case 'unauthorized':
      return `${providerName} API 키가 401 Unauthorized 응답으로 거부되었습니다. 키를 다시 확인해 주세요.`
    case 'forbidden':
      return `${providerName} 키는 인식되었지만 선택한 모델이 허용되지 않았습니다. 다른 모델을 선택하거나 플랜을 확인해 주세요.`
    case 'network_error':
    default:
      return `${providerName} 제공자에 연결하지 못했습니다. 네트워크 상태를 확인한 뒤 다시 시도해 주세요.`
  }
}

function SettingsView() {
  const i18n = useI18n()
  const steamTexts = i18n.settings.steam
  const limitTexts = i18n.settings.limits
  const retryTexts = i18n.settings.retry
  const formatDetectedNote = useMemo(
    () =>
      (path: string) =>
        steamTexts.noteDetected.replace('{path}', path),
    [steamTexts.noteDetected],
  )
  const { steamPath, detectSteamPath, scanLibrary, isScanning, libraries, error: libraryError } =
    useLibraryContext()
  const [explicitPath, setExplicitPath] = useState(steamPath ?? '')
  const [pathNote, setPathNote] = useState('')
  const [scanStatus, setScanStatus] = useState('')
  const [editingProvider, setEditingProvider] = useState<string | null>(null)
  const [draftApiKey, setDraftApiKey] = useState('')
  const [apiKeyMessage, setApiKeyMessage] = useState('')
  const [apiKeyError, setApiKeyError] = useState<string | null>(null)
  const [apiKeyStatus, setApiKeyStatus] = useState<Record<string, 'saved' | 'removed'>>({})
  const {
    selectedProviders,
    activeProviderId,
    apiKeys,
    providerModels,
    setProviderEnabled,
    updateApiKey,
    providerModelOptions,
    verifiedModels,
    keyValidation,
    validationInFlight,
    modelDiscoveryState,
    providerModelNotices,
    retryPolicy,
    revalidateProviderKey,
    concurrency,
    workerCount,
    bucketSize,
    refillMs,
    autoTuneConcurrencyOn429,
    enableBackendLogging,
    enforcePlaceholderGuard,
    prioritizeDllResources,
    enableQualitySampling,
    useServerHints,
    providerRetryPolicies,
    setConcurrency,
    setWorkerCount,
    setBucketSize,
    setRefillMs,
    updateRetryPolicy,
    setAutoTuneConcurrencyOn429,
    setEnableBackendLogging,
    setEnforcePlaceholderGuard,
    setPrioritizeDllResources,
    setEnableQualitySampling,
    setUseServerHints,
    validationMode,
    setValidationMode,
    setProviderModel,
    setProviderRetryPolicy,
  } = useSettingsStore()
  const activeProviderKey = activeProviderId ? apiKeys[activeProviderId] ?? '' : ''
  const activeProviderKeyMissing = Boolean(activeProviderId) && !activeProviderKey.trim()
  const activeProviderName = useMemo(
    () => providers.find((provider) => provider.id === activeProviderId)?.name ?? activeProviderId?.toUpperCase() ?? '선택한 번역기',
    [activeProviderId],
  )
  const libraryNotes = useMemo(
    () =>
      libraries.flatMap((library) =>
        library.notes.map((note, index) => ({
          id: `${library.path}:${index}`,
          note,
          path: library.path,
        })),
      ),
    [libraries],
  )

  const handleRetryPolicyNumberChange = useCallback(
    (provider: ProviderId, field: RetryPolicyField) =>
      (event: ChangeEvent<HTMLInputElement>) => {
        updateRetryPolicy(provider, { [field]: Number(event.target.value) } as Partial<RetryPolicy>)
      },
    [updateRetryPolicy],
  )

  const handleRetryableErrorToggle = useCallback(
    (provider: ProviderId, code: RetryableErrorCode) => {
      const current = retryPolicy[provider]?.retryableErrors ?? []
      const nextSet = new Set(current)
      if (nextSet.has(code)) {
        nextSet.delete(code)
      } else {
        nextSet.add(code)
      }
      const ordered = RETRYABLE_ERROR_CODES.filter((item) => nextSet.has(item))
      updateRetryPolicy(provider, { retryableErrors: ordered })
    },
    [retryPolicy, updateRetryPolicy],
  )

  useEffect(() => {
    setExplicitPath(steamPath ?? '')

    if (!steamPath) {
      setPathNote('')
      return
    }

    const trimmed = steamPath.trim()
    if (trimmed) {
      setPathNote(formatDetectedNote(trimmed))
    } else {
      setPathNote(steamTexts.noteNotFound)
    }
  }, [steamPath, formatDetectedNote, steamTexts.noteNotFound])

  const handleDetect = async () => {
    setScanStatus('')
    try {
      const detected = (await detectSteamPath())?.trim() ?? ''
      setExplicitPath(detected)

      if (detected) {
        setPathNote(formatDetectedNote(detected))
        setScanStatus(steamTexts.scanning)
        try {
          await scanLibrary(detected)
          setScanStatus(steamTexts.noteDone)
        } catch (error) {
          console.error('failed to scan detected Steam path', error)
          setScanStatus(steamTexts.noteError)
        }
      } else {
        setPathNote(steamTexts.noteNotFound)
        setScanStatus('')
      }
    } catch (error) {
      console.error('failed to detect Steam path', error)
      setPathNote(steamTexts.noteError)
      setScanStatus(steamTexts.noteError)
    }
  }

  const handleScan = async () => {
    const trimmed = explicitPath.trim()
    if (!trimmed) {
      setPathNote(steamTexts.noteEmpty)
      setScanStatus('')
      return
    }

    setPathNote(formatDetectedNote(trimmed))
    setScanStatus(steamTexts.scanning)
    try {
      await scanLibrary(trimmed)
      setScanStatus(steamTexts.noteDone)
    } catch (error) {
      console.error('failed to scan Steam library', error)
      setScanStatus(steamTexts.noteError)
    }
  }

  const handleStartEditing = useCallback(
    (providerId: string) => {
      setEditingProvider(providerId)
      setDraftApiKey(apiKeys[providerId] ?? '')
      setApiKeyMessage('')
      setApiKeyError(null)
      setApiKeyStatus((prev) => {
        const next = { ...prev }
        delete next[providerId]
        return next
      })
    },
    [apiKeys],
  )

  const handleApiKeyChange = useCallback((value: string) => {
    setDraftApiKey(value)
    setApiKeyError(null)
  }, [])

  const handleApiKeySave = useCallback(
    (providerId: string, providerName: string) => {
      const trimmed = draftApiKey.trim()

      try {
        updateApiKey(providerId as ProviderId, trimmed)
        setEditingProvider(null)
        setDraftApiKey('')
        setApiKeyMessage(`${providerName} API 키가 저장되었습니다.`)
        setApiKeyError(null)
        setApiKeyStatus((prev) => ({ ...prev, [providerId]: 'saved' }))
      } catch (error) {
        console.error('failed to persist API keys', error)
        setApiKeyError('API 키 저장에 실패했습니다. 저장소 접근 권한을 확인하세요.')
      }
    },
    [draftApiKey, updateApiKey],
  )

  const handleApiKeyRemove = useCallback(
    (providerId: string, providerName: string) => {
      try {
        updateApiKey(providerId as ProviderId, null)
        if (editingProvider === providerId) {
          setEditingProvider(null)
          setDraftApiKey('')
        }
        setApiKeyMessage(`${providerName} API 키가 제거되었습니다.`)
        setApiKeyError(null)
        setApiKeyStatus((prev) => ({ ...prev, [providerId]: 'removed' }))
      } catch (error) {
        console.error('failed to remove API key', error)
        setApiKeyError('API 키를 제거하는 중 문제가 발생했습니다. 다시 시도해 주세요.')
      }
    },
    [editingProvider, updateApiKey],
  )

  const handleApiKeyCancel = useCallback(() => {
    setEditingProvider(null)
    setDraftApiKey('')
    setApiKeyMessage('')
    setApiKeyError(null)
  }, [])

  return (
    <div className="space-y-8">
      <header>
        <h2 className="text-xl font-semibold text-white">작업 공간 설정</h2>
        <p className="text-sm text-slate-400">
          번역 엔진, Steam 연동, 그리고 처리량 제한을 조정할 수 있습니다. 이 화면은 Rust 백엔드와의 연계를 고려한 자리 표시자 UI입니다.
        </p>
      </header>

      <form className="space-y-6">
        <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30">
          <h3 className="text-lg font-semibold text-white">AI 제공자</h3>
          <p className="mt-1 text-sm text-slate-400">번역 작업에 사용할 모델을 선택하세요.</p>
          <div className="mt-4 grid gap-4 sm:grid-cols-2">
            {providers.map((provider) => (
              <label
                key={provider.name}
                className="flex items-center gap-3 rounded-xl border border-slate-800/60 bg-slate-950/60 p-4"
              >
                <input
                  type="checkbox"
                  checked={selectedProviders.includes(provider.id)}
                  onChange={(event) => setProviderEnabled(provider.id, event.target.checked)}
                  className="h-4 w-4 rounded border-slate-700 bg-slate-900"
                  value={provider.id}
                />
                <span>
                  <span className="block text-sm font-medium text-white">{provider.name}</span>
                  <span className="text-xs text-slate-400">{provider.description}</span>
                </span>
              </label>
            ))}
          </div>
        </section>

        <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30">
          <h3 className="text-lg font-semibold text-white">API 키 설정</h3>
          <p className="mt-1 text-sm text-slate-400">
            각 제공자별 API 키를 직접 입력해 Rust 백엔드와의 연동을 준비하세요. 빈 값으로 저장하면 키가 제거됩니다.
          </p>
          {activeProviderKeyMissing && (
            <div className="mt-4 rounded-xl border border-rose-500/40 bg-rose-500/10 px-4 py-3 text-xs text-rose-200">
              {activeProviderName} API 키가 설정되지 않았습니다. 번역 작업을 예약하기 전에 키를 입력하세요.
            </div>
          )}
          <div className="mt-4 space-y-4">
            {providers.map((provider) => {
              const storedValue = apiKeys[provider.id] ?? ''
              const isEditing = editingProvider === provider.id
              const modelOptions = providerModelOptions[provider.id] ?? []
              const verifiedList = verifiedModels?.[provider.id] ?? []
              const fallbackModels = modelOptions.filter((model) => !verifiedList.includes(model))
              const storedSelection = providerModels[provider.id]?.trim() ?? ''
              const availableOptions = modelOptions
              const selectedModel = availableOptions.includes(storedSelection)
                ? storedSelection
                : verifiedList[0] ?? fallbackModels[0] ?? ''
              const validation = keyValidation[provider.id] ?? null
              const checking = validationInFlight[provider.id] ?? false
              const discovery = modelDiscoveryState[provider.id] ?? {
                source: 'fallback',
                networkError: false,
              }
              const modelNotice = providerModelNotices[provider.id]
              const { label, className } = statusChip(validation, checking)
              const statusMessage = formatValidationMessage(
                provider.name,
                validation,
                verifiedList,
                selectedModel,
                checking,
              )
              const badge =
                discovery.source === 'live'
                  ? null
                  : discovery.networkError
                    ? {
                        text: '네트워크 오류',
                        className: 'bg-rose-700/70 text-rose-100 border-rose-400/40',
                      }
                    : {
                        text: '대체 목록',
                        className: 'bg-amber-700/70 text-amber-100 border-amber-400/40',
                      }
              const dropdownNote = (() => {
                if (checking) {
                  return '키 상태를 확인하는 중입니다. 잠시만 기다려 주세요.'
                }
                if (discovery.networkError) {
                  return '네트워크 오류로 기본 모델 목록을 사용합니다. 실행 시 실패할 수 있습니다.'
                }
                if (availableOptions.length === 0) {
                  return '표시할 모델이 없습니다. 키를 확인하거나 다른 모델 ID를 수동으로 검증해 주세요.'
                }
                if (verifiedList.length > 0) {
                  return '이 키로 확인된 모델이 먼저 표시됩니다. 아래의 기타 모델은 추가 검증이 필요할 수 있습니다.'
                }
                return '아직 검증된 모델이 없어 알려진 기본 모델 목록을 표시합니다. 사용 전 키를 확인해 주세요.'
              })()
              const retryPolicy = providerRetryPolicies[provider.id]
              const handleRetryNumberChange = (field: RetryNumericField) =>
                (event: ChangeEvent<HTMLInputElement>) => {
                  const value = Number(event.target.value)
                  setProviderRetryPolicy(
                    provider.id,
                    { [field]: value } as Partial<ProviderRetryPolicy>,
                  )
                }
              const handleRetryToggleChange = (field: RetryBooleanField) =>
                (event: ChangeEvent<HTMLInputElement>) => {
                  setProviderRetryPolicy(
                    provider.id,
                    { [field]: event.target.checked } as Partial<ProviderRetryPolicy>,
                  )
                }
              return (
                <div
                  key={provider.id}
                  className="rounded-xl border border-slate-800/60 bg-slate-950/40 p-4"
                >
                  <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                    <div>
                      <span className="block text-sm font-medium text-white">
                        {provider.name} API 키
                      </span>
                      <span className="text-xs text-slate-400">{provider.description}</span>
                      <span
                        className={`mt-2 inline-flex w-fit items-center rounded-full border px-2 py-1 text-[10px] font-semibold uppercase tracking-wider ${className}`}
                      >
                        {label}
                      </span>
                    </div>
                    <div className="flex flex-col gap-3 sm:items-end">
                      <label className="flex flex-col gap-2 text-xs text-slate-300 sm:text-sm">
                        <span className="flex items-center gap-2">
                          <span>모델</span>
                          {badge && (
                            <span
                              className={`inline-flex items-center rounded-full border px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider ${badge.className}`}
                            >
                              {badge.text}
                            </span>
                          )}
                        </span>
                        <select
                          value={selectedModel}
                          disabled={availableOptions.length === 0}
                          onChange={(event) => {
                            const nextValue = event.target.value
                            setProviderModel(provider.id, nextValue)
                            if (!checking) {
                              void revalidateProviderKey(provider.id, undefined, nextValue)
                            }
                          }}
                          className={`w-full rounded-xl border bg-slate-950 px-3 py-2 text-xs text-slate-100 focus:outline-none focus:ring-1 focus:ring-brand-500 sm:w-56 sm:text-sm ${
                            discovery.source === 'live'
                              ? 'border-slate-800 focus:border-brand-500'
                              : discovery.networkError
                                ? 'border-rose-500/60 focus:border-rose-400'
                                : 'border-amber-500/60 focus:border-amber-400'
                          }`}
                        >
                          {availableOptions.length === 0 && (
                            <option value="" disabled>
                              사용 가능한 모델이 없습니다
                            </option>
                          )}
                          {verifiedList.length > 0 && (
                            <optgroup label="이 키로 확인된 모델">
                              {verifiedList.map((model) => (
                                <option key={model} value={model}>
                                  {model}
                                </option>
                              ))}
                            </optgroup>
                          )}
                          {fallbackModels.length > 0 && (
                            <optgroup label={verifiedList.length > 0 ? '기타 알려진 모델' : '알려진 모델'}>
                              {fallbackModels.map((model) => (
                                <option key={model} value={model}>
                                  {model}
                                </option>
                              ))}
                            </optgroup>
                          )}
                        </select>
                        <p
                          className={`text-[11px] leading-relaxed ${
                            discovery.source === 'live'
                              ? 'text-slate-500'
                              : discovery.networkError
                                ? 'text-rose-200'
                                : 'text-amber-200'
                          }`}
                        >
                          {dropdownNote}
                        </p>
                        {modelNotice && (
                          <p className="text-[11px] text-amber-200">{modelNotice}</p>
                        )}
                        <p className="text-[11px] text-slate-400">{statusMessage}</p>
                      </label>
                      <button
                        type="button"
                        onClick={async () => {
                          if (checking) return
                          await revalidateProviderKey(provider.id, undefined, selectedModel)
                        }}
                        disabled={checking}
                        className="rounded-xl border border-slate-700 px-3 py-2 text-[11px] font-semibold text-slate-200 transition hover:border-brand-500 hover:text-brand-200 disabled:cursor-not-allowed disabled:opacity-60"
                      >
                        {checking ? '확인 중…' : '키 확인 / 모델 새로고침'}
                      </button>
                      <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:gap-3">
                        {isEditing ? (
                          <>
                            <input
                              type="password"
                              autoComplete="off"
                              spellCheck={false}
                              value={draftApiKey}
                              onChange={(event) => handleApiKeyChange(event.target.value)}
                              placeholder={`${provider.name} API 키를 입력하세요`}
                              className="w-full rounded-xl border border-slate-800 bg-slate-950 px-4 py-3 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500 sm:w-64"
                            />
                            <div className="flex gap-2">
                              <button
                                type="button"
                                onClick={() => handleApiKeySave(provider.id, provider.name)}
                                className="inline-flex items-center justify-center rounded-xl bg-brand-600 px-4 py-2 text-sm font-semibold text-white shadow shadow-brand-600/40 transition hover:bg-brand-500"
                              >
                                저장
                              </button>
                              <button
                                type="button"
                                onClick={handleApiKeyCancel}
                                className="inline-flex items-center justify-center rounded-xl border border-slate-700 px-4 py-2 text-sm font-semibold text-slate-200 transition hover:border-slate-500 hover:text-white"
                              >
                                취소
                              </button>
                            </div>
                          </>
                        ) : (
                          <>
                            <p className="text-xs text-slate-400 sm:text-sm">
                              {storedValue
                                ? `저장된 키: ${maskApiKey(storedValue)}`
                                : '저장된 키가 없습니다.'}
                            </p>
                            <div className="flex items-center gap-3">
                              <div className="flex gap-2">
                                <button
                                  type="button"
                                  onClick={() => handleStartEditing(provider.id)}
                                  className="inline-flex items-center justify-center rounded-xl border border-slate-700 px-4 py-2 text-sm font-semibold text-slate-200 transition hover:border-brand-500 hover:text-brand-200"
                                >
                                  {storedValue ? '수정' : '추가'}
                                </button>
                                {storedValue && (
                                  <button
                                    type="button"
                                    onClick={() => handleApiKeyRemove(provider.id, provider.name)}
                                    className="inline-flex items-center justify-center rounded-xl border border-rose-500/60 px-4 py-2 text-sm font-semibold text-rose-200 transition hover:border-rose-400 hover:text-rose-100"
                                  >
                                    제거
                                  </button>
                                )}
                              </div>
                              {apiKeyStatus[provider.id] && (
                                <span
                                  className={`text-[11px] ${
                                    apiKeyStatus[provider.id] === 'saved'
                                      ? 'text-emerald-300'
                                      : 'text-slate-400'
                                  }`}
                                >
                                  {apiKeyStatus[provider.id] === 'saved' ? '저장됨' : '제거됨'}
                                </span>
                              )}
                            </div>
                          </>
                        )}
                      </div>
                    </div>
                  </div>
                  <div className="mt-5 rounded-xl border border-slate-800/60 bg-slate-950/60 p-4">
                    <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between">
                      <span className="text-[11px] font-semibold uppercase tracking-wide text-slate-200">
                        {retryTexts.title}
                      </span>
                      <p className="text-[11px] text-slate-500 sm:text-right">
                        {retryTexts.description}
                      </p>
                    </div>
                    <div className="mt-3 grid gap-3 sm:grid-cols-2">
                      <label className="flex flex-col gap-1 text-xs text-slate-300">
                        <span>{retryTexts.fields.maxRetries.label}</span>
                        <input
                          type="number"
                          min={0}
                          step={1}
                          value={retryPolicy.maxRetries}
                          onChange={handleRetryNumberChange('maxRetries')}
                          className="rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-xs text-slate-100 focus:border-brand-500 focus:ring-brand-500 sm:text-sm"
                        />
                        <p className="text-[11px] text-slate-500">
                          {retryTexts.fields.maxRetries.hint}
                        </p>
                      </label>
                      <label className="flex flex-col gap-1 text-xs text-slate-300">
                        <span>{retryTexts.fields.initialDelayMs.label}</span>
                        <input
                          type="number"
                          min={0}
                          step={50}
                          value={retryPolicy.initialDelayMs}
                          onChange={handleRetryNumberChange('initialDelayMs')}
                          className="rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-xs text-slate-100 focus:border-brand-500 focus:ring-brand-500 sm:text-sm"
                        />
                        <p className="text-[11px] text-slate-500">
                          {retryTexts.fields.initialDelayMs.hint}
                        </p>
                      </label>
                      <label className="flex flex-col gap-1 text-xs text-slate-300">
                        <span>{retryTexts.fields.multiplier.label}</span>
                        <input
                          type="number"
                          min={1}
                          step={0.1}
                          value={retryPolicy.multiplier}
                          onChange={handleRetryNumberChange('multiplier')}
                          className="rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-xs text-slate-100 focus:border-brand-500 focus:ring-brand-500 sm:text-sm"
                        />
                        <p className="text-[11px] text-slate-500">
                          {retryTexts.fields.multiplier.hint}
                        </p>
                      </label>
                      <label className="flex flex-col gap-1 text-xs text-slate-300">
                        <span>{retryTexts.fields.maxDelayMs.label}</span>
                        <input
                          type="number"
                          min={retryPolicy.initialDelayMs}
                          step={50}
                          value={retryPolicy.maxDelayMs}
                          onChange={handleRetryNumberChange('maxDelayMs')}
                          className="rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-xs text-slate-100 focus:border-brand-500 focus:ring-brand-500 sm:text-sm"
                        />
                        <p className="text-[11px] text-slate-500">
                          {retryTexts.fields.maxDelayMs.hint}
                        </p>
                      </label>
                    </div>
                    <div className="mt-3 space-y-3 text-xs text-slate-300">
                      <div>
                        <label className="flex items-center justify-between gap-3">
                          <span>{retryTexts.toggles.respectServerHints.label}</span>
                          <input
                            type="checkbox"
                            checked={retryPolicy.respectServerRetryAfter}
                            onChange={handleRetryToggleChange('respectServerRetryAfter')}
                            className="h-4 w-4 rounded border-slate-700 bg-slate-900"
                          />
                        </label>
                        <p className="mt-1 text-[11px] text-slate-500">
                          {retryTexts.toggles.respectServerHints.hint}
                        </p>
                      </div>
                      <div>
                        <label className="flex items-center justify-between gap-3">
                          <span>{retryTexts.toggles.autoTune429.label}</span>
                          <input
                            type="checkbox"
                            checked={retryPolicy.autoTuneConcurrencyOn429}
                            onChange={handleRetryToggleChange('autoTuneConcurrencyOn429')}
                            className="h-4 w-4 rounded border-slate-700 bg-slate-900"
                          />
                        </label>
                        <p className="mt-1 text-[11px] text-slate-500">
                          {retryTexts.toggles.autoTune429.hint}
                        </p>
                      </div>
                    </div>
                  </div>
                  <p className="mt-2 text-[11px] text-slate-500">
                    API 키는 로컬 장치에 암호화되지 않은 상태로 저장되므로 보안에 유의하세요.
                  </p>
                </div>
              )
            })}
            {(apiKeyMessage || apiKeyError) && (
              <div className="space-y-1 text-xs">
                {apiKeyMessage && <p className="text-slate-400">{apiKeyMessage}</p>}
                {apiKeyError && <p className="text-rose-300">{apiKeyError}</p>}
              </div>
            )}
          </div>
        </section>

        <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30">
          <h3 className="text-lg font-semibold text-white">{steamTexts.title}</h3>
          <div className="mt-4 space-y-4">
            <label className="block text-sm font-medium text-slate-300">{steamTexts.pathLabel}</label>
            <input
              type="text"
              value={explicitPath}
              onChange={(event) => setExplicitPath(event.target.value)}
              placeholder="예: C:/Program Files (x86)/Steam"
              className="w-full rounded-xl border border-slate-800 bg-slate-950 px-4 py-3 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
            />
            {pathNote && <p className="text-xs text-slate-400">{pathNote}</p>}
            {scanStatus && <p className="text-xs text-slate-500">{scanStatus}</p>}
            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                onClick={handleDetect}
                className="inline-flex items-center justify-center rounded-xl bg-slate-800 px-4 py-2 text-sm font-semibold text-white transition hover:bg-slate-700"
              >
                {steamTexts.detect}
              </button>
              <button
                type="button"
                onClick={handleScan}
                disabled={isScanning}
                className="inline-flex items-center justify-center rounded-xl bg-brand-600 px-4 py-2 text-sm font-semibold text-white shadow shadow-brand-600/40 transition hover:bg-brand-500 disabled:cursor-not-allowed disabled:opacity-60"
              >
                {isScanning ? steamTexts.scanning : steamTexts.scan}
              </button>
            </div>
            {libraryError && (
              <p className="text-xs text-rose-300">{libraryError}</p>
            )}
            {libraryNotes.length > 0 && (
              <div className="rounded-lg border border-slate-800/60 bg-slate-950/40 p-3 text-xs text-slate-300">
                <p className="font-semibold text-slate-200">최근 스캔 메모</p>
                <ul className="mt-1 space-y-1">
                  {libraryNotes.map((item) => (
                    <li key={item.id}>
                      <span className="font-mono text-[11px] text-slate-500">{item.path}</span>{' '}
                      {item.note}
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        </section>

        <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30">
          <h3 className="text-lg font-semibold text-white">{limitTexts.title}</h3>
          <p className="mt-1 text-sm text-slate-400">번역 큐와 토큰 버킷을 조정해 공급자 제한을 준수하세요.</p>
          <div className="mt-4 grid gap-4 md:grid-cols-2 xl:grid-cols-4">
            <label className="flex flex-col gap-2 text-sm text-slate-300">
              <span>{limitTexts.concurrency}</span>
              <input
                type="number"
                value={concurrency}
                min={1}
                onChange={(event) => setConcurrency(Number(event.target.value))}
                className="rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
              />
              <p className="text-xs text-slate-500">{limitTexts.hints.concurrency}</p>
            </label>
            <label className="flex flex-col gap-2 text-sm text-slate-300">
              <span>{limitTexts.workers}</span>
              <input
                type="number"
                value={workerCount}
                min={1}
                onChange={(event) => setWorkerCount(Number(event.target.value))}
                className="rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
              />
              <p className="text-xs text-slate-500">{limitTexts.hints.workers}</p>
            </label>
            <label className="flex flex-col gap-2 text-sm text-slate-300">
              <span>{limitTexts.bucket}</span>
              <input
                type="number"
                value={bucketSize}
                min={1}
                onChange={(event) => setBucketSize(Number(event.target.value))}
                className="rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
              />
              <p className="text-xs text-slate-500">{limitTexts.hints.bucket}</p>
            </label>
            <label className="flex flex-col gap-2 text-sm text-slate-300">
              <span>{limitTexts.refillMs}</span>
              <input
                type="number"
                value={refillMs}
                min={100}
                step={50}
                onChange={(event) => setRefillMs(Number(event.target.value))}
                className="rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
              />
              <p className="text-xs text-slate-500">{limitTexts.hints.refillMs}</p>
            </label>
            <label className="md:col-span-2 xl:col-span-4 flex items-center justify-between gap-3 rounded-xl border border-slate-800 bg-slate-950 px-4 py-3 text-sm text-slate-300">
              <span>429 응답 시 동시성 자동 조정</span>
              <input
                type="checkbox"
                checked={autoTuneConcurrencyOn429}
                onChange={(event) => setAutoTuneConcurrencyOn429(event.target.checked)}
                className="h-4 w-4 rounded border-slate-700 bg-slate-900"
              />
            </label>
          </div>
        </section>

        <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30">
          <h3 className="text-lg font-semibold text-white">재시도 정책</h3>
          <p className="mt-2 text-sm text-slate-400">
            제공자별 최대 시도 횟수와 지연 시간을 조정하고, 재시도 대상 오류를 선택할 수 있습니다.
          </p>
          <div className="mt-4 space-y-4">
            {providers.map((provider) => {
              const policy = retryPolicy[provider.id]
              if (!policy) {
                return null
              }
              return (
                <div
                  key={provider.id}
                  className="space-y-4 rounded-xl border border-slate-800/60 bg-slate-950/30 p-4"
                >
                  <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                    <div>
                      <p className="text-sm font-semibold text-white">{provider.name}</p>
                      <p className="text-xs text-slate-400">{provider.description}</p>
                    </div>
                    <div className="grid grid-cols-1 gap-3 text-xs text-slate-300 sm:grid-cols-3">
                      <label className="flex flex-col gap-1">
                        <span className="text-[11px] uppercase tracking-wide text-slate-500">최대 시도 횟수</span>
                        <input
                          type="number"
                          min={1}
                          value={policy.maxAttempts}
                          onChange={handleRetryPolicyNumberChange(provider.id, 'maxAttempts')}
                          className="rounded-lg border border-slate-800 bg-slate-950 px-2 py-1 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
                        />
                      </label>
                      <label className="flex flex-col gap-1">
                        <span className="text-[11px] uppercase tracking-wide text-slate-500">초기 지연(ms)</span>
                        <input
                          type="number"
                          min={0}
                          step={50}
                          value={policy.initialDelayMs}
                          onChange={handleRetryPolicyNumberChange(provider.id, 'initialDelayMs')}
                          className="rounded-lg border border-slate-800 bg-slate-950 px-2 py-1 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
                        />
                      </label>
                      <label className="flex flex-col gap-1">
                        <span className="text-[11px] uppercase tracking-wide text-slate-500">최대 지연(ms)</span>
                        <input
                          type="number"
                          min={0}
                          step={50}
                          value={policy.maxDelayMs}
                          onChange={handleRetryPolicyNumberChange(provider.id, 'maxDelayMs')}
                          className="rounded-lg border border-slate-800 bg-slate-950 px-2 py-1 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
                        />
                      </label>
                    </div>
                  </div>
                  <div>
                    <p className="text-[11px] uppercase tracking-wide text-slate-500">재시도 대상 오류</p>
                    <div className="mt-2 flex flex-wrap gap-3">
                      {RETRYABLE_ERROR_CODES.map((code) => {
                        const checked = policy.retryableErrors.includes(code)
                        return (
                          <label
                            key={code}
                            className={`flex items-center gap-2 rounded-full border px-3 py-1 text-xs transition ${
                              checked
                                ? 'border-brand-500/60 bg-brand-500/10 text-brand-100'
                                : 'border-slate-700 bg-slate-900/50 text-slate-300'
                            }`}
                          >
                            <input
                              type="checkbox"
                              checked={checked}
                              onChange={() => handleRetryableErrorToggle(provider.id, code)}
                              className="h-3.5 w-3.5 rounded border-slate-700 bg-slate-900"
                            />
                            <span>{RETRYABLE_ERROR_LABELS[code] ?? code}</span>
                          </label>
                        )
                      })}
                    </div>
                  </div>
                </div>
              )
            })}
          </div>
          <div className="mt-4 space-y-2 text-sm text-slate-300">
            <label className="flex items-center justify-between gap-3">
              <span>{limitTexts.autoTune}</span>
              <input
                type="checkbox"
                checked={autoTuneConcurrencyOn429}
                onChange={(event) => setAutoTuneConcurrencyOn429(event.target.checked)}
                className="h-4 w-4 rounded border-slate-700 bg-slate-900"
              />
            </label>
            <p className="text-xs text-slate-500">{limitTexts.hints.autoTune}</p>
          </div>
        </section>

        <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30">
          <h3 className="text-lg font-semibold text-white">번역 규칙 및 로깅</h3>
          <div className="mt-4 space-y-4 text-sm text-slate-300">
            <label className="flex items-center justify-between gap-3">
              <span>백엔드 상세 로그 남기기</span>
              <input
                type="checkbox"
                checked={enableBackendLogging}
                onChange={(event) => setEnableBackendLogging(event.target.checked)}
                className="h-4 w-4 rounded border-slate-700 bg-slate-900"
              />
            </label>
            <label className="flex items-center justify-between gap-3">
              <span>서버 재시도 힌트 우선 사용</span>
              <input
                type="checkbox"
                checked={useServerHints}
                onChange={(event) => setUseServerHints(event.target.checked)}
                className="h-4 w-4 rounded border-slate-700 bg-slate-900"
              />
            </label>
            <label className="flex items-center justify-between gap-3">
              <span>플레이스홀더 일치 검증 강제</span>
              <input
                type="checkbox"
                checked={enforcePlaceholderGuard}
                onChange={(event) => setEnforcePlaceholderGuard(event.target.checked)}
                className="h-4 w-4 rounded border-slate-700 bg-slate-900"
              />
            </label>
            <label className="flex items-center justify-between gap-3">
              <span>검증 모드</span>
              <select
                value={validationMode}
                onChange={(event) => setValidationMode(event.target.value as import('../storage/settingsStorage').ValidationMode)}
                className="rounded border border-slate-700 bg-slate-900 px-3 py-1.5 text-sm text-slate-200 focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500"
              >
                <option value="strict">엄격 (Strict)</option>
                <option value="relaxed_xml">느슨 (Relaxed XML)</option>
              </select>
            </label>
            <p className="text-xs text-slate-400">
              느슨 모드: 수식/라텍스 무시, XML 태그 경계 내 텍스트만 검증, 자동 복구 활성화 (권장)
            </p>
            <label className="flex items-center justify-between gap-3">
              <span>DLL 리소스 우선 처리 (Mono.Cecil)</span>
              <input
                type="checkbox"
                checked={prioritizeDllResources}
                onChange={(event) => setPrioritizeDllResources(event.target.checked)}
                className="h-4 w-4 rounded border-slate-700 bg-slate-900"
              />
            </label>
            <label className="flex items-center justify-between gap-3">
              <span>품질 샘플링(5%) 수행</span>
              <input
                type="checkbox"
                checked={enableQualitySampling}
                onChange={(event) => setEnableQualitySampling(event.target.checked)}
                className="h-4 w-4 rounded border-slate-700 bg-slate-900"
              />
            </label>
          </div>
        </section>
      </form>
    </div>
  )
}

export default SettingsView
