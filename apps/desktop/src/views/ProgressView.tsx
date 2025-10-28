import { memo, useCallback, useEffect, useMemo, useState } from 'react'
import { Link } from 'react-router-dom'
import { useJobStore, type JobFileEntry } from '../context/JobStore'
import type { JobState } from '../types/core'

const stateLabels: Record<JobState, string> = {
  queued: '대기 중',
  running: '실행 중',
  completed: '완료',
  failed: '실패',
  canceled: '취소됨',
}

const stateClasses: Record<JobState, string> = {
  queued: 'bg-slate-800 text-slate-300',
  running: 'bg-brand-500/20 text-brand-200',
  completed: 'bg-emerald-500/20 text-emerald-200',
  failed: 'bg-rose-500/20 text-rose-100',
  canceled: 'bg-slate-700/60 text-slate-300',
}

const progressClasses: Record<JobState, string> = {
  queued: 'bg-slate-600',
  running: 'bg-brand-500',
  completed: 'bg-emerald-500',
  failed: 'bg-rose-500',
  canceled: 'bg-slate-700',
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

const DEFAULT_SOURCE_LANGUAGE = 'en'
const DEFAULT_TARGET_LANGUAGE = 'ko'

const EMPTY_JOB_FILES: JobFileEntry[] = []
const EMPTY_SELECTED_FILES: string[] = []

function resolveLanguageLabel(code: string) {
  const normalized = code.toLowerCase()
  return languageBadges[normalized] ?? normalized.toUpperCase()
}

function ProgressView() {
  const {
    currentJob,
    queue,
    appendLog,
    markCurrentJobFailed,
    loadFilesForCurrentJob,
    toggleCurrentJobFileSelection,
    startTranslationForCurrentJob,
  } = useJobStore()
  const [selectionError, setSelectionError] = useState<string | null>(null)
  const [isStarting, setIsStarting] = useState(false)
  const activeJobId = currentJob?.jobId ?? null
  const shouldLoadFiles = Boolean(
    activeJobId && !currentJob?.files && !currentJob?.filesLoading && !currentJob?.fileError,
  )

  useEffect(() => {
    setSelectionError(null)
    setIsStarting(false)
  }, [activeJobId])

  useEffect(() => {
    if (!activeJobId || !shouldLoadFiles) {
      return
    }

    loadFilesForCurrentJob().catch((error) => {
      console.error('파일 목록을 불러오는 중 오류가 발생했습니다.', error)
    })
  }, [activeJobId, shouldLoadFiles, loadFilesForCurrentJob])

  const handleToggleFile = useCallback(
    (path: string) => {
      if (!activeJobId) return
      toggleCurrentJobFileSelection(path)
      setSelectionError(null)
    },
    [activeJobId, toggleCurrentJobFileSelection],
  )

  const files: JobFileEntry[] = currentJob?.files ?? EMPTY_JOB_FILES
  const fileLoading = currentJob?.filesLoading ?? false
  const fileError = currentJob?.fileError ?? null
  const selectedFilePaths = currentJob?.selectedFiles ?? EMPTY_SELECTED_FILES
  const sourceLanguageGuess = currentJob?.sourceLanguageGuess ?? DEFAULT_SOURCE_LANGUAGE

  const autoSelectedCount = useMemo(
    () => files.filter((entry) => entry.autoSelected).length,
    [files],
  )

  const isJobExecuting = Boolean(currentJob?.backendJobId) && currentJob?.status === 'running'

  const handleStart = useCallback(async () => {
    if (!currentJob) return
    if (fileLoading || isJobExecuting || isStarting) {
      return
    }
    if (!selectedFilePaths.length) {
      setSelectionError('번역할 파일을 하나 이상 선택해 주세요.')
      return
    }

    setSelectionError(null)
    setIsStarting(true)
    try {
      appendLog(`${currentJob.modName} 번역을 시작합니다.`)
      await startTranslationForCurrentJob({
        selectedFiles: selectedFilePaths,
        sourceLanguageGuess,
        targetLanguage: currentJob.targetLanguage ?? DEFAULT_TARGET_LANGUAGE,
      })
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setSelectionError(message)
      appendLog(message, 'error')
      markCurrentJobFailed(message)
    } finally {
      setIsStarting(false)
    }
  }, [appendLog, currentJob, fileLoading, isJobExecuting, isStarting, markCurrentJobFailed, selectedFilePaths, sourceLanguageGuess, startTranslationForCurrentJob])

  if (!currentJob) {
    return (
      <div className="mx-auto flex max-w-3xl flex-col items-center justify-center gap-6 rounded-2xl border border-slate-800/60 bg-slate-900/60 p-10 text-center text-slate-300">
        <div className="text-2xl font-semibold text-white">진행 중인 작업이 없습니다.</div>
        <p className="text-sm text-slate-400">
          모드 관리 화면에서 번역할 모드를 선택하면 작업이 대기열에 추가되고 이곳에서 진행 상황을 확인할 수 있습니다.
        </p>
        <Link
          to="/mods"
          className="inline-flex items-center justify-center rounded-full bg-brand-600 px-4 py-2 text-sm font-semibold text-white shadow shadow-brand-600/40 transition hover:bg-brand-500"
        >
          모드 관리로 이동
        </Link>
      </div>
    )
  }

  const stateClass = stateClasses[currentJob.status]
  const progressBarClass = progressClasses[currentJob.status]
  const clampedProgress = Math.max(0, Math.min(100, Math.round(currentJob.progress)))
  const disableSelection = fileLoading || isJobExecuting || isStarting
  const startDisabled = disableSelection || !selectedFilePaths.length

  return (
    <div className="space-y-6">
      <header className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <div>
          <h2 className="text-xl font-semibold text-white">번역 진행 상황</h2>
          <p className="text-sm text-slate-400">
            현재 활성화된 작업에 대한 진행률과 로그를 표시합니다. 대기 중인 작업은 자동으로 이어집니다.
          </p>
        </div>
        <span className={`rounded-full px-3 py-1 text-xs font-semibold ${stateClass}`}>
          {stateLabels[currentJob.status]}
        </span>
      </header>

      <section className="space-y-4 rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30">
        <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <div>
            <h3 className="text-lg font-semibold text-white">{currentJob.modName}</h3>
            <p className="text-xs uppercase tracking-wider text-slate-500">{currentJob.workshopId}</p>
            <p className="mt-1 text-sm text-slate-400">게임: {currentJob.gameName}</p>
          </div>
          <div className="text-right text-xs text-slate-400">
            <p>진행률 {clampedProgress}%</p>
            <p>대기열 잔여 {queue.length}건</p>
          </div>
        </div>

        <div className="space-y-3 text-sm text-slate-300">
          <p>{currentJob.message ?? '번역 작업을 시작하면 진행 메시지가 여기에 표시됩니다.'}</p>
          <div className="flex items-center gap-3 text-xs text-slate-400">
            <div className="h-2 flex-1 overflow-hidden rounded-full bg-slate-800/60">
              <div
                className={`h-full rounded-full transition-all duration-300 ${progressBarClass}`}
                style={{ width: `${clampedProgress}%` }}
              />
            </div>
            <span className="w-12 text-right text-slate-300">{clampedProgress}%</span>
          </div>
          {currentJob.translator && (
            <p className="text-xs text-slate-500">번역기: {currentJob.translator}</p>
          )}
          {currentJob.preview && (
            <div className="rounded-lg border border-slate-800/60 bg-slate-950/40 p-3 text-xs text-slate-300">
              <p className="font-semibold text-slate-200">미리보기</p>
              <p className="mt-1 whitespace-pre-wrap text-slate-300">{currentJob.preview}</p>
            </div>
          )}
        </div>
      </section>

      <section className="space-y-4 rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6">
        <div className="flex flex-col gap-1 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h4 className="text-sm font-semibold text-slate-200">번역 대상 파일</h4>
            <p className="text-xs text-slate-400">
              자동으로 감지된 언어 파일이 선택됩니다. 필요에 따라 번역할 파일을 조정하세요.
            </p>
          </div>
          {fileLoading && <span className="text-xs text-slate-400">파일 불러오는 중...</span>}
        </div>

        {fileError ? (
          <p className="text-xs text-rose-300">{fileError}</p>
        ) : (
          <>
            <div className="max-h-72 overflow-y-auto rounded-lg border border-slate-800/60 bg-slate-900/60">
              {files.length ? (
                <ul className="divide-y divide-slate-800/60">
                  {files.map((entry) => (
                    <FileRow
                      key={entry.path}
                      entry={entry}
                      disableSelection={disableSelection}
                      onToggle={handleToggleFile}
                    />
                  ))}
                </ul>
              ) : (
                <div className="p-4 text-xs text-slate-400 sm:text-sm">
                  {fileLoading ? '파일을 불러오는 중입니다.' : '표시할 텍스트 파일이 없습니다.'}
                </div>
              )}
            </div>

            {!fileLoading && !autoSelectedCount && files.length > 0 && (
              <p className="text-xs text-amber-200">
                알려진 언어 파일을 찾지 못했습니다. 번역할 파일을 수동으로 선택해 주세요.
              </p>
            )}

            {selectionError && <p className="text-xs text-rose-300">{selectionError}</p>}

            <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
              <div className="text-xs text-slate-400">
                선택된 파일 {selectedFilePaths.length}개 · 추정 원본 언어 {sourceLanguageGuess.toUpperCase()}
              </div>
              <button
                type="button"
                onClick={handleStart}
                disabled={startDisabled}
                className="inline-flex items-center justify-center rounded-full bg-brand-600 px-4 py-2 text-sm font-semibold text-white shadow shadow-brand-600/40 transition hover:bg-brand-500 disabled:cursor-not-allowed disabled:opacity-60"
              >
                {isJobExecuting ? '번역 진행 중' : isStarting ? '준비 중...' : '번역 시작'}
              </button>
            </div>
          </>
        )}
      </section>

      <section className="space-y-4 rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6">
        <h4 className="text-sm font-semibold text-slate-200">실시간 로그</h4>
        {currentJob.logs.length ? (
          <ul className="space-y-2 text-xs text-slate-300">
            {currentJob.logs.map((entry) => (
              <li
                key={entry.id}
                className={`rounded-lg border border-slate-800/60 bg-slate-950/40 px-3 py-2 ${
                  entry.level === 'error' ? 'border-rose-500/40 text-rose-200' : ''
                }`}
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="font-medium text-slate-200">
                    {new Date(entry.timestamp).toLocaleTimeString()}
                  </span>
                  <span className="text-[10px] uppercase tracking-widest text-slate-500">{entry.level}</span>
                </div>
                <p className="mt-1 whitespace-pre-wrap">{entry.message}</p>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-xs text-slate-400">아직 표시할 로그가 없습니다.</p>
        )}
      </section>
    </div>
  )
}

interface FileRowProps {
  entry: JobFileEntry
  disableSelection: boolean
  onToggle: (path: string) => void
}

const FileRow = memo(
  ({ entry, disableSelection, onToggle }: FileRowProps) => {
    const isChecked = entry.selected
    return (
      <li
        className={`px-4 py-2 text-xs text-slate-300 transition sm:text-sm ${isChecked ? 'bg-slate-800/40' : ''}`}
      >
        <label className="flex cursor-pointer items-start justify-between gap-3">
          <div className="flex items-start gap-3">
            <input
              type="checkbox"
              checked={isChecked}
              onChange={() => onToggle(entry.path)}
              disabled={disableSelection}
              className="mt-0.5 h-4 w-4 rounded border-slate-700 bg-slate-900 text-brand-500 focus:ring-brand-500 disabled:cursor-not-allowed disabled:opacity-60"
            />
            <span className="font-mono text-[11px] text-slate-200 sm:text-xs md:text-sm">{entry.path}</span>
          </div>
          <div className="flex items-center gap-2">
            {entry.languageHint && (
              <span className="rounded-full border border-slate-700 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wide text-slate-200">
                {resolveLanguageLabel(entry.languageHint)}
              </span>
            )}
            {entry.autoSelected && (
              <span className="rounded-full border border-emerald-500/40 px-2 py-0.5 text-[10px] font-semibold text-emerald-300">
                자동
              </span>
            )}
          </div>
        </label>
      </li>
    )
  },
  (previous, next) =>
    previous.entry === next.entry &&
    previous.disableSelection === next.disableSelection &&
    previous.onToggle === next.onToggle,
)

export default ProgressView

