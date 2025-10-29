import { useCallback, useEffect, useMemo, useState } from 'react'
import { useLibraryContext } from '../context/LibraryContext'
import { useI18n } from '../i18n/ko'
import { maskApiKey } from '../storage/apiKeyStorage'
import { useSettingsStore, type ProviderId } from '../context/SettingsStore'

const providers: Array<{ id: ProviderId; name: string; description: string }> = [
  {
    id: 'gemini',
    name: '제미니',
    description: 'Google 기반 컨텍스트 확장 모델을 사용합니다.',
  },
  {
    id: 'gpt',
    name: 'gpt',
    description: '긴 컨텍스트와 안정적인 번역 품질을 제공합니다.',
  },
  {
    id: 'claude',
    name: '클로드',
    description: '대사 중심 콘텐츠에 적합한 Anthropic 어댑터입니다.',
  },
  {
    id: 'grok',
    name: '그록',
    description: '신속한 반복 실험에 적합한 실험적 제공자입니다.',
  },
]

function SettingsView() {
  const i18n = useI18n()
  const steamTexts = i18n.settings.steam
  const limitTexts = i18n.settings.limits
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
    setProviderEnabled,
    updateApiKey,
    concurrency,
    workerCount,
    bucketSize,
    refillMs,
    enableBackendLogging,
    enforcePlaceholderGuard,
    prioritizeDllResources,
    enableQualitySampling,
    setConcurrency,
    setWorkerCount,
    setBucketSize,
    setRefillMs,
    setEnableBackendLogging,
    setEnforcePlaceholderGuard,
    setPrioritizeDllResources,
    setEnableQualitySampling,
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
              return (
                <div
                  key={provider.id}
                  className="rounded-xl border border-slate-800/60 bg-slate-950/40 p-4"
                >
                  <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
                    <div>
                      <span className="block text-sm font-medium text-white">
                        {provider.name} API 키
                      </span>
                      <span className="text-xs text-slate-400">{provider.description}</span>
                    </div>
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
              <span>플레이스홀더 일치 검증 강제</span>
              <input
                type="checkbox"
                checked={enforcePlaceholderGuard}
                onChange={(event) => setEnforcePlaceholderGuard(event.target.checked)}
                className="h-4 w-4 rounded border-slate-700 bg-slate-900"
              />
            </label>
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
