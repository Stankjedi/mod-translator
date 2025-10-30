import { memo, useCallback, useEffect, useMemo, useState, type ChangeEvent } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Link } from 'react-router-dom'
import {
  useJobStore,
  type JobFileEntry,
  providerLabelFor,
} from '../context/JobStore'
import { useToast } from '../context/ToastStore'
import {
  useSettingsStore,
  type KeyValidationState,
  type ProviderId,
} from '../context/SettingsStore'
import type { JobState } from '../types/core'
import Chip, { type ChipTone } from '../ui/Chip'

const isTauriRuntime = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

const statusLabels: Record<JobState, string> = {
  pending: '준비 중',
  running: '진행 중',
  completed: '완료됨',
  failed: '실패',
  canceled: '중단됨',
  partial_success: '부분 성공',
}

const statusTones: Record<JobState, ChipTone> = {
  pending: 'idle',
  running: 'running',
  completed: 'primary',
  failed: 'error',
  canceled: 'warning',
  partial_success: 'warning',
}

const progressClasses: Record<JobState, string> = {
  pending: 'bg-slate-600',
  running: 'bg-brand-500',
  completed: 'bg-emerald-500',
  failed: 'bg-rose-500',
  canceled: 'bg-slate-700',
  partial_success: 'bg-amber-500',
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

const KNOWN_PROVIDERS: ProviderId[] = ['gemini', 'gpt', 'claude', 'grok']

function isKnownProviderId(value: string): value is ProviderId {
  return KNOWN_PROVIDERS.includes(value as ProviderId)
}

function resolveLanguageLabel(code: string) {
  const normalized = code.toLowerCase()
  return languageBadges[normalized] ?? normalized.toUpperCase()
}

function ProgressView() {
  const {
    currentJob,
    queue,
    completedJobs,
    appendLog,
    loadFilesForCurrentJob,
    toggleCurrentJobFileSelection,
    startTranslationForCurrentJob,
    requestCancelCurrentJob,
    updateCurrentJobTargetLanguage,
    updateCurrentJobOutputOverride,
    dismissCurrentJob,
  } = useJobStore()
  const { showToast } = useToast()
  const { keyValidation } = useSettingsStore()
  const [selectionError, setSelectionError] = useState<string | null>(null)
  const [isStarting, setIsStarting] = useState(false)
  const [cancelError, setCancelError] = useState<string | null>(null)
  const [openError, setOpenError] = useState<string | null>(null)
  const [targetLanguageDraft, setTargetLanguageDraft] = useState(DEFAULT_TARGET_LANGUAGE)
  const [outputOverrideDraft, setOutputOverrideDraft] = useState('')
  const activeJobId = currentJob?.id ?? null
  const shouldLoadFiles = Boolean(
    activeJobId && !currentJob?.files && !currentJob?.filesLoading && !currentJob?.fileListError,
  )

  useEffect(() => {
    setSelectionError(null)
    setIsStarting(false)
    setCancelError(null)
    setOpenError(null)
  }, [activeJobId])

  useEffect(() => {
    setTargetLanguageDraft(currentJob?.targetLanguage ?? DEFAULT_TARGET_LANGUAGE)
  }, [currentJob?.targetLanguage])

  useEffect(() => {
    setOutputOverrideDraft(currentJob?.outputOverrideDir ?? '')
  }, [currentJob?.outputOverrideDir])

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
  const fileListError = currentJob?.fileListError ?? null
  const selectedFilePaths = currentJob?.selectedFiles ?? EMPTY_SELECTED_FILES
  const sourceLanguageGuess = currentJob?.sourceLanguageGuess ?? DEFAULT_SOURCE_LANGUAGE
  const targetLanguage = currentJob?.targetLanguage ?? DEFAULT_TARGET_LANGUAGE
  const overridePreview = currentJob?.outputOverrideDir?.trim()
  const outputPath = currentJob
    ? currentJob.outputPath?.trim() || overridePreview || currentJob.installPath
    : ''

  const autoSelectedCount = useMemo(
    () => files.filter((entry) => entry.autoSelected).length,
    [files],
  )

  const isJobExecuting = currentJob?.status === 'running'
  const sourceLanguageLabel = resolveLanguageLabel(sourceLanguageGuess)
  const targetLanguageLabel = resolveLanguageLabel(targetLanguage)
  const historyEntries = useMemo(() => [...completedJobs].slice(-10).reverse(), [completedJobs])
  const currentJobFailedFiles = useMemo(
    () => (currentJob ? currentJob.fileErrors : []),
    [currentJob],
  )

  const handleStart = useCallback(async () => {
    if (!currentJob || currentJob.status !== 'pending') return
    if (fileLoading || isJobExecuting || isStarting || currentJob.cancelRequested) {
      return
    }
    if (!selectedFilePaths.length) {
      setSelectionError('번역할 파일을 하나 이상 선택해 주세요.')
      return
    }

    setSelectionError(null)
    setIsStarting(true)
    const resolvedTarget = targetLanguageDraft.trim() || DEFAULT_TARGET_LANGUAGE
    try {
      appendLog(`${currentJob.modName} 번역을 시작합니다.`)
      await startTranslationForCurrentJob({
        selectedFiles: selectedFilePaths,
        sourceLanguageGuess,
        targetLanguage: resolvedTarget,
      })
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setSelectionError(message)
      appendLog(message, 'error')
    } finally {
      setIsStarting(false)
    }
  }, [
    appendLog,
    currentJob,
    fileLoading,
    isJobExecuting,
    isStarting,
    selectedFilePaths,
    sourceLanguageGuess,
    startTranslationForCurrentJob,
    targetLanguageDraft,
  ])

  const handleCancel = useCallback(async () => {
    setCancelError(null)
    const result = await requestCancelCurrentJob()
    if (!result.success) {
      setCancelError('작업 중단 요청에 실패했습니다. 잠시 후 다시 시도해 주세요.')
      return
    }

    if (result.previousStatus === 'pending') {
      showToast('Preparing job canceled.', 'neutral')
    }
  }, [requestCancelCurrentJob, showToast])

  const handleTargetLanguageChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const value = event.target.value
      setTargetLanguageDraft(value)
      updateCurrentJobTargetLanguage(value)
    },
    [updateCurrentJobTargetLanguage],
  )

  const handleOutputOverrideChange = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => {
      const value = event.target.value
      setOutputOverrideDraft(value)
      updateCurrentJobOutputOverride(value)
    },
    [updateCurrentJobOutputOverride],
  )

  const handleOpenOutput = useCallback(async () => {
    if (!currentJob) return
    setOpenError(null)
    const path = (currentJob.outputPath ?? currentJob.installPath)?.trim()
    if (!path) {
      setOpenError('출력 경로 정보를 찾을 수 없습니다.')
      return
    }

    if (!isTauriRuntime()) {
      setOpenError('출력 폴더 열기는 데스크톱 환경에서만 지원됩니다.')
      return
    }

    try {
      await invoke('open_output_folder', { path })
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      setOpenError(message)
    }
  }, [currentJob])

  const handleOpenHistoryOutput = useCallback(async (path: string) => {
    if (!path.trim() || !isTauriRuntime()) {
      return
    }
    try {
      await invoke('open_output_folder', { path })
    } catch (error) {
      console.error('완료된 작업 출력 폴더를 여는 중 오류가 발생했습니다.', error)
    }
  }, [])

  const startWarning = useMemo(() => {
    if (!currentJob || currentJob.status !== 'pending') {
      return null
    }
    const providerId = currentJob.providerId
    if (!providerId || !isKnownProviderId(providerId)) {
      return null
    }
    const status: KeyValidationState | null = keyValidation[providerId] ?? null
    if (status === 'valid') {
      return null
    }
    const providerName = providerLabelFor(providerId)
    switch (status) {
      case 'unauthorized':
        return `${providerName} API 키가 "인증 실패" 상태입니다. 설정에서 키를 다시 확인하지 않으면 번역이 401 오류로 중단될 수 있습니다.`
      case 'forbidden':
        return `${providerName} API 키가 "권한 없음" 상태입니다. 계정 권한을 조정한 뒤 다시 시도하세요.`
      case 'network_error':
        return `${providerName} API 키 상태를 확인하지 못했습니다(네트워크 오류). 오프라인 상태에서는 번역이 실패할 수 있습니다.`
      case null:
      default:
        return `${providerName} API 키 상태가 확인되지 않았습니다. 설정 화면에서 "키 정상" 상태인지 확인한 뒤 번역을 시작하세요.`
    }
  }, [currentJob, keyValidation])

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

  const isPending = currentJob.status === 'pending'
  const isRunning = currentJob.status === 'running'
  const isCancelRequested = currentJob.cancelRequested
  const statusLabel =
    isCancelRequested && (isRunning || isPending)
      ? '중단 요청됨…'
      : statusLabels[currentJob.status]
  const statusTone =
    isCancelRequested && (isRunning || isPending)
      ? 'warning'
      : statusTones[currentJob.status]
  const progressBarClass = progressClasses[currentJob.status]
  const clampedProgress = Math.max(0, Math.min(100, Math.round(currentJob.progress)))
  const isTerminalState =
    currentJob.status === 'completed' ||
    currentJob.status === 'failed' ||
    currentJob.status === 'canceled' ||
    currentJob.status === 'partial_success'
  const disableSelection =
    fileLoading ||
    isJobExecuting ||
    isStarting ||
    currentJob.cancelRequested ||
    currentJob.status !== 'pending'
  const startDisabled =
    disableSelection || currentJob.status !== 'pending' || !selectedFilePaths.length
  const showCancelButton = isPending || isRunning
  const showDismissButton = isTerminalState
  const cancelButtonDisabled = isCancelRequested || (!isPending && !isRunning)
  const cancelButtonLabel = isCancelRequested
    ? isPending
      ? '취소 요청됨…'
      : '중단 요청됨…'
    : isPending
      ? '취소'
      : '중단'
  const cancelButtonClasses = isPending
    ? 'inline-flex items-center justify-center rounded-full border border-slate-600/70 px-4 py-2 text-sm font-semibold text-slate-200 transition hover:border-slate-500 hover:text-white disabled:cursor-not-allowed disabled:opacity-60'
    : 'inline-flex items-center justify-center rounded-full bg-rose-600 px-4 py-2 text-sm font-semibold text-white shadow shadow-rose-600/30 transition hover:bg-rose-500 disabled:cursor-not-allowed disabled:opacity-60'
  const cancelButtonTitle = isPending ? 'Cancel preparing job' : 'Cancel running job'
  const providerDisplay = providerLabelFor(currentJob.providerId)
  const modelDisplay = currentJob.modelId
  const translatedSummary =
    currentJob.totalCount > 0
      ? `${currentJob.translatedCount} / ${currentJob.totalCount}개 번역됨`
      : null
  const startButtonLabel =
    currentJob.status === 'running' ? '번역 진행 중' : isStarting ? '준비 중...' : '번역 시작'
  const showTargetLanguageEditor = currentJob.status === 'pending'
  const latestLogText = currentJob.logs.length
    ? currentJob.logs[currentJob.logs.length - 1].text
    : '번역 작업을 시작하면 진행 메시지가 여기에 표시됩니다.'

  return (
    <div className="space-y-6">
      <header className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between">
        <div>
          <h2 className="text-xl font-semibold text-white">번역 진행 상황</h2>
          <p className="text-sm text-slate-400">
            현재 활성화된 작업에 대한 진행률과 로그를 표시합니다. 현재 작업을 닫으면 대기열에 있는 다음 작업이 준비됩니다.
          </p>
          {cancelError && <p className="mt-2 text-xs text-rose-300">{cancelError}</p>}
        </div>
        <div className="flex items-center gap-3">
          <Chip label={`번역기: ${providerDisplay}`} tone="idle" />
          {modelDisplay && <Chip label={`모델: ${modelDisplay}`} tone="idle" />}
          <Chip label={statusLabel} tone={statusTone} />
          {showCancelButton && (
            <button
              type="button"
              onClick={handleCancel}
              disabled={cancelButtonDisabled}
              className={cancelButtonClasses}
              title={cancelButtonTitle}
              aria-label={cancelButtonTitle}
            >
              {cancelButtonLabel}
            </button>
          )}
          {showDismissButton && (
            <button
              type="button"
              onClick={dismissCurrentJob}
              className="inline-flex items-center justify-center rounded-full border border-slate-700 px-4 py-2 text-sm font-semibold text-slate-200 transition hover:border-brand-400 hover:text-white"
            >
              작업 닫기
            </button>
          )}
        </div>
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

        <div className="space-y-4 text-sm text-slate-300">
          <p>{latestLogText}</p>
          <div className="flex items-center gap-3 text-xs text-slate-400">
            <div className="h-2 flex-1 overflow-hidden rounded-full bg-slate-800/60">
              <div
                className={`h-full rounded-full transition-all duration-300 ${progressBarClass}`}
                style={{ width: `${clampedProgress}%` }}
              />
            </div>
            <span className="w-12 text-right text-slate-300">{clampedProgress}%</span>
          </div>
          <div className="grid gap-2 text-xs text-slate-500 sm:grid-cols-2">
            {providerDisplay && <p>번역기: {providerDisplay}</p>}
            {modelDisplay && <p>모델: {modelDisplay}</p>}
            <p>언어: {sourceLanguageLabel} → {targetLanguageLabel}</p>
            {translatedSummary && <p>{translatedSummary}</p>}
            <p>선택된 파일 {selectedFilePaths.length}개</p>
          </div>
          {currentJobFailedFiles.length > 0 && (
            <div className="rounded-lg border border-rose-500/40 bg-rose-950/20 p-3 text-xs text-rose-100">
              <p className="text-[11px] font-semibold uppercase tracking-wider text-rose-200">
                오류가 발생한 파일
              </p>
              <ul className="mt-2 space-y-2">
                {currentJobFailedFiles.map((entry) => {
                  const key = `${entry.filePath}|${entry.message}|${entry.code ?? ''}`
                  return (
                    <li key={key} className="space-y-1">
                      <p className="font-mono text-[11px] text-rose-100 sm:text-xs">{entry.filePath}</p>
                      <p className="whitespace-pre-wrap text-[11px] text-rose-100/90 sm:text-xs">
                        {entry.message}
                      </p>
                      {entry.code && (
                        <p className="text-[10px] uppercase tracking-widest text-rose-300/90">코드: {entry.code}</p>
                      )}
                    </li>
                  )
                })}
              </ul>
            </div>
          )}
          {outputPath && (
            <div className="flex flex-col gap-2 text-xs text-slate-500 sm:flex-row sm:items-center sm:gap-3">
              <span>
                출력 경로:{' '}
                <span className="font-mono text-[11px] text-slate-300 sm:text-xs">{outputPath}</span>
              </span>
              {isTauriRuntime() && (
                <button
                  type="button"
                  onClick={handleOpenOutput}
                  className="inline-flex w-fit items-center justify-center rounded-full border border-slate-700 px-3 py-1 text-[11px] font-semibold text-slate-200 transition hover:border-brand-400 hover:text-white"
                >
                  폴더 열기
                </button>
              )}
            </div>
          )}
          {openError && <p className="text-xs text-rose-300">{openError}</p>}
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

        {fileListError ? (
          <p className="text-xs text-rose-300">{fileListError}</p>
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

            <div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <div className="text-xs text-slate-400">
                선택된 파일 {selectedFilePaths.length}개 · 추정 원본 언어 {sourceLanguageLabel} · 목표 언어 {targetLanguageLabel}
              </div>
              <div className="flex flex-col items-stretch gap-2 sm:flex-row sm:items-center sm:gap-3">
                <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:gap-3 sm:flex-1">
                  {showTargetLanguageEditor && (
                    <label className="flex items-center gap-2 text-xs text-slate-300">
                      <span>목표 언어</span>
                      <input
                        type="text"
                        value={targetLanguageDraft}
                        onChange={handleTargetLanguageChange}
                        className="w-24 rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200 focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500"
                      />
                    </label>
                  )}
                  <label className="flex flex-1 items-center gap-2 text-xs text-slate-300">
                    <span>출력 폴더</span>
                    <input
                      type="text"
                      value={outputOverrideDraft}
                      onChange={handleOutputOverrideChange}
                      disabled={currentJob?.status !== 'pending'}
                      placeholder="비워두면 원본 파일 옆에 저장"
                      className="flex-1 rounded border border-slate-700 bg-slate-900 px-2 py-1 text-xs text-slate-200 focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500 disabled:cursor-not-allowed disabled:opacity-60"
                    />
                  </label>
                </div>
                {startWarning && (
                  <p className="text-[11px] text-rose-200 sm:text-xs">{startWarning}</p>
                )}
                <button
                  type="button"
                  onClick={handleStart}
                  disabled={startDisabled}
                  className="inline-flex items-center justify-center rounded-full bg-brand-600 px-4 py-2 text-sm font-semibold text-white shadow shadow-brand-600/40 transition hover:bg-brand-500 disabled:cursor-not-allowed disabled:opacity-60"
                >
                  {startButtonLabel}
                </button>
              </div>
            </div>
          </>
        )}
      </section>

      <section className="space-y-4 rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6">
        <h4 className="text-sm font-semibold text-slate-200">실시간 로그</h4>
        {currentJob.logs.length ? (
          <ul className="space-y-2 text-xs text-slate-300">
            {currentJob.logs.map((entry, index) => (
              <li
                key={`${entry.ts}-${index}`}
                className={`rounded-lg border border-slate-800/60 bg-slate-950/40 px-3 py-2 ${
                  entry.level === 'error'
                    ? 'border-rose-500/40 text-rose-200'
                    : entry.level === 'warn'
                      ? 'border-amber-500/40 text-amber-100'
                      : ''
                }`}
              >
                <div className="flex items-center justify-between gap-2">
                  <span className="font-medium text-slate-200">
                    {new Date(entry.ts).toLocaleTimeString()}
                  </span>
                  <span className="text-[10px] uppercase tracking-widest text-slate-500">{entry.level}</span>
                </div>
                <p className="mt-1 whitespace-pre-wrap">{entry.text}</p>
              </li>
            ))}
          </ul>
        ) : (
          <p className="text-xs text-slate-400">아직 표시할 로그가 없습니다.</p>
        )}
      </section>

      <section className="space-y-4 rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6">
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <h4 className="text-sm font-semibold text-slate-200">완료된 작업 기록</h4>
          {historyEntries.length > 0 && (
            <span className="text-xs text-slate-500">최근 {historyEntries.length}건</span>
          )}
        </div>
        {historyEntries.length ? (
          <ul className="space-y-3 text-xs text-slate-300">
            {historyEntries.map((job) => {
              const historyProvider = providerLabelFor(job.providerId)
              const historyModel = job.modelId
              const historyStatusLabel = statusLabels[job.status]
              const historyTone = statusTones[job.status]
              const counts =
                job.totalCount > 0
                  ? `${job.translatedCount} / ${job.totalCount}개 번역됨`
                  : `진행률 ${Math.round(job.progress)}%`
              const completionTime = new Date(job.completedAt ?? job.createdAt).toLocaleString()
              const historySource = resolveLanguageLabel(job.sourceLanguageGuess ?? DEFAULT_SOURCE_LANGUAGE)
              const historyTarget = resolveLanguageLabel(job.targetLanguage ?? DEFAULT_TARGET_LANGUAGE)
              const finalLog = job.logs.length ? job.logs[job.logs.length - 1].text : null
              const failedFileEntries = job.fileErrors
              return (
                <li key={job.id} className="rounded-xl border border-slate-800/60 bg-slate-900/60 p-4">
                  <div className="flex flex-col gap-3 md:flex-row md:items-start md:justify-between">
                    <div className="space-y-1">
                      <div className="flex flex-wrap items-center gap-2 text-sm font-semibold text-white">
                        <span>{job.modName}</span>
                        <Chip label={historyProvider} tone="idle" />
                        {historyModel && <Chip label={historyModel} tone="idle" />}
                        <Chip label={historyStatusLabel} tone={historyTone} />
                      </div>
                      <p className="text-xs text-slate-400">완료 {completionTime}</p>
                      {historyModel && (
                        <p className="text-xs text-slate-500">모델: {historyModel}</p>
                      )}
                      <p className="text-xs text-slate-500">
                        언어: {historySource} → {historyTarget}
                      </p>
                      {counts && <p className="text-xs text-slate-500">{counts}</p>}
                      {failedFileEntries.length > 0 && (
                        <div className="space-y-2 rounded-lg border border-rose-500/40 bg-rose-950/10 p-3 text-[11px] text-rose-100 sm:text-xs">
                          <p className="font-semibold uppercase tracking-wider text-rose-200">
                            실패한 파일
                          </p>
                          <ul className="space-y-1">
                            {failedFileEntries.map((entry) => {
                              const key = `${entry.filePath}|${entry.message}|${entry.code ?? ''}`
                              return (
                                <li key={key} className="space-y-1">
                                  <span className="block font-mono text-rose-100">{entry.filePath}</span>
                                  <span className="block whitespace-pre-wrap text-rose-100/90">{entry.message}</span>
                                  {entry.code && (
                                    <span className="block text-[10px] uppercase tracking-widest text-rose-300/90">
                                      코드: {entry.code}
                                    </span>
                                  )}
                                </li>
                              )
                            })}
                          </ul>
                        </div>
                      )}
                      <div className="flex flex-col gap-2 text-xs text-slate-500 sm:flex-row sm:items-center sm:gap-3">
                        <span className="font-mono text-[11px] text-slate-300 sm:text-xs">
                          출력 경로: {job.outputPath}
                        </span>
                        {isTauriRuntime() && (
                          <button
                            type="button"
                            onClick={() => handleOpenHistoryOutput(job.outputPath)}
                            className="inline-flex w-fit items-center justify-center rounded-full border border-slate-700 px-3 py-1 text-[11px] font-semibold text-slate-200 transition hover:border-brand-400 hover:text-white"
                          >
                            폴더 열기
                          </button>
                        )}
                      </div>
                    </div>
                    {finalLog && <p className="max-w-md text-xs text-slate-400">{finalLog}</p>}
                  </div>
                </li>
              )
            })}
          </ul>
        ) : (
          <p className="text-xs text-slate-400">아직 완료된 작업 기록이 없습니다.</p>
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

