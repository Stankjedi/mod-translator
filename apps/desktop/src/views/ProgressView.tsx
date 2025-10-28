import { useCallback, useEffect, useMemo, useState } from 'react'
import { Link, useNavigate, useParams } from 'react-router-dom'
import { invoke } from '@tauri-apps/api/core'
import { useLibraryContext } from '../context/LibraryContext'
import { useJobContext } from '../context/JobContext'
import type { JobState, ModFileDescriptor, ModFileListing, ModSummary } from '../types/core'

const stateLabels: Record<JobState, string> = {
  queued: '대기 중',
  running: '실행 중',
  completed: '완료',
  failed: '실패',
}

const stateClasses: Record<JobState, string> = {
  queued: 'bg-slate-800 text-slate-300',
  running: 'bg-brand-500/20 text-brand-200',
  completed: 'bg-emerald-500/20 text-emerald-200',
  failed: 'bg-rose-500/20 text-rose-100',
}

const progressClasses: Record<JobState, string> = {
  queued: 'bg-slate-600',
  running: 'bg-brand-500',
  completed: 'bg-emerald-500',
  failed: 'bg-rose-500',
}

const languageBadges: Record<string, string> = {
  en: 'EN',
  'zh-cn': 'ZH-CN',
  'zh-tw': 'ZH-TW',
  ja: 'JA',
  ru: 'RU',
  fr: 'FR',
  de: 'DE',
  es: 'ES',
  pt: 'PT',
  pl: 'PL',
  it: 'IT',
  ko: 'KO',
}

const languagePriority = ['en', 'zh-cn', 'zh-tw', 'ja', 'ru', 'fr', 'de', 'es', 'pt', 'pl', 'it', 'ko']

const DEFAULT_SOURCE_LANGUAGE = 'en'

const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

function resolveLanguageLabel(code: string) {
  const normalized = code.toLowerCase()
  return languageBadges[normalized] ?? normalized.toUpperCase()
}

function guessSourceLanguage(entries: ModFileDescriptor[]) {
  const hints = entries
    .map((entry) => entry.language_hint?.toLowerCase())
    .filter((hint): hint is string => Boolean(hint))

  for (const candidate of languagePriority) {
    if (hints.includes(candidate)) {
      return candidate
    }
  }

  if (hints.length > 0) {
    return hints[0]
  }

  return DEFAULT_SOURCE_LANGUAGE
}

function ProgressView() {
  const { libraries } = useLibraryContext()
  const { jobsByMod, startJob, refreshJob } = useJobContext()
  const [pendingMods, setPendingMods] = useState<Record<string, boolean>>({})
  const { modId: encodedModId } = useParams<{ modId?: string }>()
  const navigate = useNavigate()
  const [fileEntries, setFileEntries] = useState<ModFileDescriptor[]>([])
  const [fileLoading, setFileLoading] = useState(false)
  const [fileError, setFileError] = useState<string | null>(null)
  const [selectedPaths, setSelectedPaths] = useState<Set<string>>(new Set())
  const [selectionError, setSelectionError] = useState<string | null>(null)

  const selectedModId = useMemo(() => {
    if (!encodedModId) return null
    try {
      return decodeURIComponent(encodedModId)
    } catch {
      return encodedModId
    }
  }, [encodedModId])

  const trackedMods = useMemo(
    () => libraries.flatMap((library) => library.mods),
    [libraries],
  )

  const focusedMod = useMemo(
    () => (selectedModId ? trackedMods.find((mod) => mod.id === selectedModId) ?? null : null),
    [selectedModId, trackedMods],
  )

  const displayedMods = useMemo(() => {
    if (!selectedModId) return trackedMods
    return focusedMod ? [focusedMod] : []
  }, [focusedMod, selectedModId, trackedMods])

  const isFocusedView = Boolean(selectedModId)
  const missingFocusedMod = Boolean(selectedModId) && !focusedMod

  useEffect(() => {
    setFileEntries([])
    setSelectedPaths(new Set())
    setSelectionError(null)
    setFileError(null)

    if (!focusedMod) {
      setFileLoading(false)
      return
    }

    if (!isTauri()) {
      setFileError('파일 목록은 데스크톱 환경에서만 확인할 수 있습니다.')
      return
    }

    const directory = focusedMod.directory?.trim()
    if (!directory) {
      setFileError('모드 디렉터리 정보를 찾을 수 없습니다.')
      return
    }

    let isCancelled = false
    setFileLoading(true)

    invoke<ModFileListing>('list_mod_files', {
      mod_directory: directory,
    })
      .then((listing) => {
        if (isCancelled) return
        setFileEntries(listing.files)
        const initialSelected = listing.files
          .filter((entry) => entry.translatable && (entry.auto_selected || false))
          .map((entry) => entry.path)
        setSelectedPaths(new Set(initialSelected))
      })
      .catch((err) => {
        if (isCancelled) return
        const message = err instanceof Error ? err.message : String(err)
        setFileError(message)
        setFileEntries([])
      })
      .finally(() => {
        if (isCancelled) return
        setFileLoading(false)
      })

    return () => {
      isCancelled = true
    }
  }, [focusedMod])

  const handleToggleFile = useCallback(
    (path: string) => {
      setSelectedPaths((prev) => {
        const next = new Set(prev)
        if (next.has(path)) {
          next.delete(path)
        } else {
          next.add(path)
        }
        return next
      })
      setSelectionError(null)
    },
    [setSelectionError],
  )

  const selectedEntries = useMemo(
    () => fileEntries.filter((entry) => selectedPaths.has(entry.path)),
    [fileEntries, selectedPaths],
  )

  const selectedFilePaths = useMemo(
    () => Array.from(selectedPaths).sort((a, b) => a.localeCompare(b)),
    [selectedPaths],
  )

  const sourceLanguageGuess = useMemo(
    () => guessSourceLanguage(selectedEntries),
    [selectedEntries],
  )

  const autoSelectedCount = useMemo(
    () => fileEntries.filter((entry) => entry.auto_selected).length,
    [fileEntries],
  )

  const setPending = useCallback((modId: string, value: boolean) => {
    setPendingMods((prev) => {
      if (value) {
        return { ...prev, [modId]: true }
      }
      const { [modId]: _removed, ...rest } = prev
      return rest
    })
  }, [])

  const handleStart = useCallback(
    async (mod: ModSummary, isFocusedMod: boolean) => {
      if (!isFocusedMod) {
        navigate(`/progress/${encodeURIComponent(mod.id)}`)
        return
      }

      if (fileLoading) {
        return
      }

      if (!selectedFilePaths.length) {
        setSelectionError('번역할 파일을 하나 이상 선택해 주세요.')
        return
      }

      setSelectionError(null)
      setPending(mod.id, true)
      try {
        await startJob(mod, {
          selectedFiles: selectedFilePaths,
          sourceLanguageGuess,
          targetLanguage: 'ko',
        })
      } catch (err) {
        console.error(err)
      } finally {
        setPending(mod.id, false)
      }
    },
    [fileLoading, navigate, selectedFilePaths, setPending, sourceLanguageGuess, startJob],
  )

  const handleRefresh = async (modId: string) => {
    setPending(modId, true)
    try {
      await refreshJob(modId)
    } finally {
      setPending(modId, false)
    }
  }

  return (
    <div className="space-y-6">
      <header className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <div>
          <h2 className="text-xl font-semibold text-white">번역 진행 상황</h2>
          <p className="text-sm text-slate-400">
            {isFocusedView
              ? '선택한 모드에 대한 번역 상태만 표시합니다. 필요한 경우 다시 전체 목록으로 돌아갈 수 있습니다.'
              : '감지된 모드별로 번역 작업을 예약하고 Rust 백엔드의 작업 큐에서 보고된 상태를 실시간으로 확인하세요.'}
          </p>
        </div>
        {isFocusedView && (
          <Link
            to="/progress"
            className="inline-flex items-center justify-center rounded-full border border-slate-700 px-4 py-2 text-sm font-semibold text-slate-200 transition hover:border-brand-500 hover:text-brand-200"
          >
            전체 모드 보기
          </Link>
        )}
      </header>

      {displayedMods.length ? (
        <div className="space-y-4">
          {displayedMods.map((mod) => {
            const jobEntry = jobsByMod[mod.id]
            const status = jobEntry?.status
            const progressValue = Math.round((status?.progress ?? 0) * 100)
            const clampedProgress = Math.max(0, Math.min(100, progressValue))
            const stateClass = status ? stateClasses[status.state] : 'bg-slate-800 text-slate-300'
            const progressBarClass = status ? progressClasses[status.state] : 'bg-slate-700'
            const isPending = Boolean(pendingMods[mod.id])
            const isRunning = status ? status.state === 'queued' || status.state === 'running' : false
            const isFocusedMod = isFocusedView && focusedMod?.id === mod.id
            const disableSelection = isPending || isRunning || fileLoading
            const startDisabled =
              isPending || isRunning || (isFocusedMod ? fileLoading || !selectedFilePaths.length : false)
            const selectedCountLabel = isFocusedMod && selectedEntries.length > 0 ? ` (${selectedEntries.length}개)` : ''
            const startLabel = isFocusedMod
              ? isRunning
                ? '진행 중'
                : `선택한 파일 번역${selectedCountLabel}`
              : '파일 선택'

            return (
              <article
                key={mod.id}
                className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30"
              >
                <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
                  <div>
                    <h3 className="text-lg font-semibold text-white">{mod.name}</h3>
                    <p className="text-xs uppercase tracking-wider text-slate-500">{mod.id}</p>
                    <p className="mt-1 text-sm text-slate-400">게임: {mod.game}</p>
                  </div>
                  <span className={`rounded-full px-3 py-1 text-xs font-semibold ${stateClass}`}>
                    {status ? stateLabels[status.state] : '대기 중'}
                  </span>
                </div>

                <div className="mt-4 space-y-3 text-sm text-slate-300">
                  <p>{status?.message ?? '번역 작업을 시작하면 진행 메시지가 여기에 표시됩니다.'}</p>
                  <div className="flex items-center gap-3 text-xs text-slate-400">
                    <div className="h-2 flex-1 overflow-hidden rounded-full bg-slate-800/60">
                      <div
                        className={`h-full rounded-full transition-all duration-300 ${progressBarClass}`}
                        style={{ width: `${clampedProgress}%` }}
                      />
                    </div>
                    <span className="w-12 text-right text-slate-300">{clampedProgress}%</span>
                  </div>
                  {status && (
                    <p className="text-xs text-slate-500">번역기: {status.translator}</p>
                  )}
                  {status?.preview && (
                    <div className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-3 text-xs text-slate-300">
                      <p className="font-semibold text-slate-200">미리보기</p>
                      <p className="mt-1 whitespace-pre-wrap text-slate-300">{status.preview}</p>
                    </div>
                  )}
                </div>

                {isFocusedMod && (
                  <div className="mt-5 rounded-xl border border-slate-800/60 bg-slate-950/40 p-4">
                    <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between">
                      <div>
                        <h4 className="text-sm font-semibold text-slate-200">번역 대상 파일</h4>
                        <p className="text-xs text-slate-400">
                          자동으로 감지된 언어 파일이 선택됩니다. 필요에 따라 번역할 파일을 수동으로 조정하세요.
                        </p>
                      </div>
                      {fileLoading && <span className="text-xs text-slate-400">파일 불러오는 중...</span>}
                    </div>

                    {fileError ? (
                      <p className="mt-3 text-xs text-rose-300">{fileError}</p>
                    ) : (
                      <>
                        <div className="mt-3 max-h-64 overflow-y-auto rounded-lg border border-slate-800/60 bg-slate-900/60">
                          {fileEntries.length ? (
                            <ul className="divide-y divide-slate-800/60">
                              {fileEntries.map((entry) => {
                                const isChecked = selectedPaths.has(entry.path)
                                return (
                                  <li
                                    key={entry.path}
                                    className={`px-4 py-2 text-xs text-slate-300 transition sm:text-sm ${
                                      isChecked ? 'bg-slate-800/40' : ''
                                    }`}
                                  >
                                    <label className="flex items-start justify-between gap-3">
                                      <div className="flex items-start gap-3">
                                        <input
                                          type="checkbox"
                                          checked={isChecked}
                                          onChange={() => handleToggleFile(entry.path)}
                                          disabled={disableSelection}
                                          className="mt-0.5 h-4 w-4 rounded border-slate-700 bg-slate-900 text-brand-500 focus:ring-brand-500 disabled:cursor-not-allowed disabled:opacity-60"
                                        />
                                        <span className="font-mono text-[11px] text-slate-200 sm:text-xs md:text-sm">
                                          {entry.path}
                                        </span>
                                      </div>
                                      <div className="flex items-center gap-2">
                                        {entry.language_hint && (
                                          <span className="rounded-full border border-slate-700 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-slate-200">
                                            {resolveLanguageLabel(entry.language_hint)}
                                          </span>
                                        )}
                                        {entry.auto_selected && (
                                          <span className="rounded-full border border-emerald-500/40 px-2 py-0.5 text-[10px] font-semibold text-emerald-300">
                                            자동
                                          </span>
                                        )}
                                      </div>
                                    </label>
                                  </li>
                                )
                              })}
                            </ul>
                          ) : (
                            <div className="p-4 text-xs text-slate-400 sm:text-sm">
                              {fileLoading ? '파일을 불러오는 중입니다.' : '표시할 텍스트 파일이 없습니다.'}
                            </div>
                          )}
                        </div>

                        {!fileLoading && !autoSelectedCount && fileEntries.length > 0 && (
                          <p className="mt-3 text-xs text-amber-200">
                            알려진 언어 파일을 찾지 못했습니다. 번역할 파일을 수동으로 선택해 주세요.
                          </p>
                        )}

                        {selectionError && (
                          <p className="mt-3 text-xs text-rose-300">{selectionError}</p>
                        )}

                        {selectedEntries.length > 0 && (
                          <p className="mt-3 text-xs text-slate-400">
                            선택된 {selectedEntries.length}개 파일 · 예상 원본 언어{' '}
                            <span className="font-semibold text-slate-200">
                              {resolveLanguageLabel(sourceLanguageGuess)}
                            </span>{' '}
                            → 한국어
                          </p>
                        )}
                      </>
                    )}
                  </div>
                )}

                <div className="mt-4 flex flex-wrap gap-2">
                  <button
                    type="button"
                    onClick={() => handleStart(mod, isFocusedMod)}
                    disabled={startDisabled}
                    className="rounded-lg bg-brand-600 px-4 py-2 text-sm font-semibold text-white shadow shadow-brand-600/40 transition hover:bg-brand-500 disabled:cursor-not-allowed disabled:opacity-60"
                  >
                    {startLabel}
                  </button>
                  <button
                    type="button"
                    onClick={() => handleRefresh(mod.id)}
                    disabled={isPending || !status}
                    className="rounded-lg border border-slate-700 px-4 py-2 text-sm font-semibold text-slate-200 transition hover:border-brand-500 hover:text-brand-200 disabled:cursor-not-allowed disabled:opacity-60"
                  >
                    상태 새로고침
                  </button>
                </div>

                {mod.warnings.length ? (
                  <div className="mt-4 rounded-lg border border-amber-500/30 bg-amber-500/5 p-3 text-xs text-amber-100">
                    <p className="font-semibold text-amber-200">감지된 경고</p>
                    <ul className="mt-1 space-y-1">
                      {mod.warnings.map((warning) => (
                        <li key={warning}>{warning}</li>
                      ))}
                    </ul>
                  </div>
                ) : (
                  <p className="mt-4 text-xs text-slate-500">추가 경고가 없습니다.</p>
                )}
                {isFocusedView && mod.policy.notes.length > 0 && (
                  <div className="mt-4 rounded-lg border border-slate-800/60 bg-slate-900/60 p-3 text-xs text-slate-300">
                    <p className="font-semibold text-slate-200">정책 참고</p>
                    <ul className="mt-1 space-y-1">
                      {mod.policy.notes.map((note) => (
                        <li key={note}>{note}</li>
                      ))}
                    </ul>
                  </div>
                )}
              </article>
            )
          })}
        </div>
      ) : (
        <div className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-10 text-center">
          <p className="text-base font-semibold text-white">
            {missingFocusedMod ? '선택한 모드를 찾지 못했습니다.' : '등록된 모드가 없습니다.'}
          </p>
          <p className="mt-2 text-sm text-slate-400">
            {missingFocusedMod
              ? '라이브러리를 다시 스캔했는지 확인하거나 다른 모드를 선택해 주세요.'
              : '라이브러리를 스캔하여 모드를 감지하면 이곳에 작업 대기열이 표시됩니다.'}
          </p>
          {missingFocusedMod && (
            <div className="mt-4 flex justify-center">
              <Link
                to="/mods"
                className="rounded-full bg-brand-600/20 px-4 py-2 text-sm font-semibold text-brand-400 transition hover:bg-brand-600/40 hover:text-brand-200"
              >
                모드 관리로 이동
              </Link>
            </div>
          )}
        </div>
      )}
    </div>
  )
}

export default ProgressView
