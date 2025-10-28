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
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import type {
  JobState,
  ModFileListing,
  ProviderId,
  TranslationProgressEventPayload,
} from '../types/core'
import { useSettingsStore } from './SettingsStore'

const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

const DEFAULT_TARGET_LANGUAGE = 'ko'
const DEFAULT_SOURCE_LANGUAGE = 'en'
const LANGUAGE_PRIORITY = [
  'en',
  'zh-cn',
  'zh-tw',
  'ja',
  'ru',
  'fr',
  'de',
  'es',
  'pt',
  'pl',
  'it',
  'ko',
]

const PROVIDER_LABELS: Record<ProviderId, string> = {
  gemini: '제미니',
  gpt: 'GPT',
  claude: 'Claude',
  grok: 'Grok',
}

const createId = () =>
  typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : `job-${Date.now()}-${Math.random().toString(16).slice(2)}`

export type JobLogLevel = 'info' | 'error'

export interface JobLogEntry {
  id: string
  level: JobLogLevel
  message: string
  timestamp: number
}

export interface JobFileEntry {
  path: string
  translatable: boolean
  autoSelected: boolean
  languageHint: string | null
  selected: boolean
}

export interface QueueJob {
  jobId: string
  modId: string
  workshopId: string
  modName: string
  gameName: string
  installPath: string
  progress: number
  status: JobState
  logs: JobLogEntry[]
  backendJobId: string | null
  message: string | null
  providerId: ProviderId | null
  providerLabel: string | null
  providerApiKey: string | null
  translatedCount: number
  totalCount: number
  selectedFiles: string[]
  sourceLanguageGuess: string | null
  targetLanguage: string | null
  lastUpdated: number
  files: JobFileEntry[] | null
  filesLoading: boolean
  fileError: string | null
  cancelRequested: boolean
}

interface JobStoreState {
  currentJob: QueueJob | null
  queue: QueueJob[]
  completedJobs: QueueJob[]
}

export interface EnqueueJobInput {
  modId: string
  workshopId?: string
  modName: string
  gameName: string
  installPath: string
}

export type EnqueueJobError = 'duplicate-active' | 'duplicate-queued' | 'invalid-path'

export interface EnqueueJobResult {
  job: QueueJob
  promoted: boolean
  error: EnqueueJobError | null
}

export interface StartTranslationOptions {
  selectedFiles: string[]
  sourceLanguageGuess: string | null
  targetLanguage?: string
}

interface JobStoreValue {
  currentJob: QueueJob | null
  queue: QueueJob[]
  completedJobs: QueueJob[]
  enqueueJob: (input: EnqueueJobInput) => EnqueueJobResult
  promoteNextJob: () => void
  appendLog: (message: string, level?: JobLogLevel) => void
  markCurrentJobCompleted: (message?: string | null, patch?: Partial<QueueJob>) => void
  markCurrentJobFailed: (message?: string | null, patch?: Partial<QueueJob>) => void
  markCurrentJobCanceled: (message?: string | null, patch?: Partial<QueueJob>) => void
  cancelQueuedJob: (jobId: string) => boolean
  loadFilesForCurrentJob: () => Promise<void>
  toggleCurrentJobFileSelection: (path: string) => void
  startTranslationForCurrentJob: (options: StartTranslationOptions) => Promise<void>
  requestCancelCurrentJob: () => Promise<boolean>
}

const JobStoreContext = createContext<JobStoreValue | undefined>(undefined)

const clampProgress = (value: number) => {
  if (Number.isNaN(value)) {
    return 0
  }
  return Math.min(100, Math.max(0, value))
}

const createLogEntry = (level: JobLogLevel, message: string): JobLogEntry => ({
  id: createId(),
  level,
  message,
  timestamp: Date.now(),
})

const guessSourceLanguageFromFiles = (entries: JobFileEntry[]): string => {
  const hints = entries
    .map((entry) => entry.languageHint?.toLowerCase())
    .filter((hint): hint is string => Boolean(hint))

  for (const candidate of LANGUAGE_PRIORITY) {
    if (hints.includes(candidate)) {
      return candidate
    }
  }

  if (hints.length > 0) {
    return hints[0]
  }

  return DEFAULT_SOURCE_LANGUAGE
}

const prepareJobForActivation = (job: QueueJob): QueueJob => ({
  ...job,
  status: 'running',
  progress: 0,
  backendJobId: null,
  message: null,
  logs: [],
  selectedFiles: [],
  sourceLanguageGuess: null,
  targetLanguage: job.targetLanguage ?? DEFAULT_TARGET_LANGUAGE,
  translatedCount: 0,
  totalCount: 0,
  lastUpdated: Date.now(),
  files: null,
  filesLoading: false,
  fileError: null,
  cancelRequested: false,
})

const createJob = (input: EnqueueJobInput): QueueJob => ({
  jobId: createId(),
  modId: input.modId,
  workshopId: input.workshopId ?? input.modId,
  modName: input.modName,
  gameName: input.gameName,
  installPath: input.installPath,
  progress: 0,
  status: 'queued',
  logs: [],
  backendJobId: null,
  message: null,
  providerId: null,
  providerLabel: null,
  providerApiKey: null,
  translatedCount: 0,
  totalCount: 0,
  selectedFiles: [],
  sourceLanguageGuess: null,
  targetLanguage: DEFAULT_TARGET_LANGUAGE,
  lastUpdated: Date.now(),
  files: null,
  filesLoading: false,
  fileError: null,
  cancelRequested: false,
})

const promoteNextJobState = (previous: JobStoreState): JobStoreState => {
  if (previous.queue.length === 0) {
    if (previous.currentJob === null) {
      return previous
    }
    return {
      ...previous,
      currentJob: null,
      queue: [],
    }
  }

  const [nextJob, ...rest] = previous.queue
  return {
    ...previous,
    currentJob: prepareJobForActivation(nextJob),
    queue: rest,
  }
}

export function JobStoreProvider({ children }: { children: ReactNode }) {
  const { activeProviderId, selectedProviders, apiKeys } = useSettingsStore()
  const [state, setState] = useState<JobStoreState>({
    currentJob: null,
    queue: [],
    completedJobs: [],
  })
  const activeJobIdRef = useRef<string | null>(null)

  useEffect(() => {
    activeJobIdRef.current = state.currentJob?.jobId ?? null
  }, [state.currentJob?.jobId])

  const promoteNextJob = useCallback(() => {
    setState((prev) => promoteNextJobState(prev))
  }, [])

  const appendLog = useCallback((message: string, level: JobLogLevel = 'info') => {
    const trimmed = message.trim()
    if (!trimmed) {
      return
    }
    const entry = createLogEntry(level, trimmed)
    setState((prev) => {
      if (!prev.currentJob) return prev
      return {
        ...prev,
        currentJob: {
          ...prev.currentJob,
          logs: [...prev.currentJob.logs, entry],
          lastUpdated: Date.now(),
        },
      }
    })
  }, [])

  const finalizeCurrentJobAndPromote = useCallback(
    (
      status: Extract<JobState, 'completed' | 'failed' | 'canceled'>,
      options: {
        message?: string | null
        patch?: Partial<QueueJob>
        level?: JobLogLevel
      } = {},
    ) => {
      setState((prev) => {
        if (!prev.currentJob) {
          return prev
        }

        const { patch, message, level } = options
        const patchValues: Partial<QueueJob> = patch ?? {}
        const normalizedMessage = message?.trim() ?? null
        const logLevel: JobLogLevel = level ?? (status === 'failed' ? 'error' : 'info')
        const baseLogs = prev.currentJob.logs
        const logEntry = normalizedMessage ? createLogEntry(logLevel, normalizedMessage) : null

        const archivedBase: QueueJob = {
          ...prev.currentJob,
          ...patchValues,
          status,
          progress:
            patchValues.progress ??
            (status === 'completed' ? 100 : prev.currentJob.progress),
          cancelRequested:
            status === 'canceled'
              ? true
              : patchValues.cancelRequested ?? prev.currentJob.cancelRequested,
          translatedCount:
            patchValues.translatedCount ?? prev.currentJob.translatedCount,
          totalCount: patchValues.totalCount ?? prev.currentJob.totalCount,
          message: patchValues.message ?? normalizedMessage ?? prev.currentJob.message,
          logs: baseLogs,
          lastUpdated: Date.now(),
        }

        const archivedJob: QueueJob = {
          ...archivedBase,
          logs: logEntry ? [...archivedBase.logs, logEntry] : archivedBase.logs,
        }

        const [nextJob, ...restQueue] = prev.queue
        const nextCurrent = nextJob ? prepareJobForActivation(nextJob) : null

        return {
          ...prev,
          currentJob: nextCurrent,
          queue: restQueue,
          completedJobs: [...prev.completedJobs, archivedJob],
        }
      })
    },
    [],
  )

  const markCurrentJobCompleted = useCallback(
    (message?: string | null, patch?: Partial<QueueJob>) => {
      finalizeCurrentJobAndPromote('completed', { message: message ?? null, patch, level: 'info' })
    },
    [finalizeCurrentJobAndPromote],
  )

  const markCurrentJobFailed = useCallback(
    (message?: string | null, patch?: Partial<QueueJob>) => {
      finalizeCurrentJobAndPromote('failed', { message: message ?? null, patch, level: 'error' })
    },
    [finalizeCurrentJobAndPromote],
  )

  const markCurrentJobCanceled = useCallback(
    (message?: string | null, patch?: Partial<QueueJob>) => {
      finalizeCurrentJobAndPromote('canceled', { message: message ?? null, patch, level: 'info' })
    },
    [finalizeCurrentJobAndPromote],
  )

  const enqueueJob = useCallback(
    (input: EnqueueJobInput): EnqueueJobResult => {
      let result: EnqueueJobResult | null = null

      const providerId = activeProviderId ?? selectedProviders[0] ?? null
      const providerLabel = providerId ? PROVIDER_LABELS[providerId] ?? providerId.toUpperCase() : null
      const providerApiKey = providerId ? apiKeys[providerId] ?? null : null

      setState((prev) => {
        if (prev.currentJob && prev.currentJob.modId === input.modId) {
          result = { job: prev.currentJob, promoted: false, error: 'duplicate-active' }
          return prev
        }

      const duplicateQueued = prev.queue.find((job) => job.modId === input.modId)
      if (duplicateQueued) {
        result = { job: duplicateQueued, promoted: false, error: 'duplicate-queued' }
        return prev
      }

      const job: QueueJob = {
        ...createJob(input),
        providerId,
        providerLabel,
        providerApiKey: providerApiKey ?? null,
      }
      const trimmedPath = job.installPath.trim()

      if (!trimmedPath) {
        const failureLog = createLogEntry(
          'error',
          '모드 설치 경로가 유효하지 않아 작업을 완료할 수 없습니다.',
        )
        const failedJob: QueueJob = {
          ...job,
          installPath: job.installPath,
          status: 'failed',
          logs: [failureLog],
          lastUpdated: Date.now(),
        }

        result = { job: failedJob, promoted: false, error: 'invalid-path' }

        return {
          ...prev,
          completedJobs: [...prev.completedJobs, failedJob],
        }
      }

      const normalizedJob: QueueJob = {
        ...job,
        installPath: trimmedPath,
        lastUpdated: Date.now(),
      }

      if (!prev.currentJob) {
        const activated = prepareJobForActivation(normalizedJob)
        result = { job: activated, promoted: true, error: null }
        return {
          ...prev,
          currentJob: activated,
        }
      }

      result = { job: normalizedJob, promoted: false, error: null }
      return {
        ...prev,
        queue: [...prev.queue, normalizedJob],
      }
    })

    if (!result) {
      throw new Error('enqueueJob 결과를 결정하지 못했습니다.')
    }

    return result
  }, [activeProviderId, apiKeys, selectedProviders])

  const cancelQueuedJob = useCallback((jobId: string) => {
    let cancelled = false
    setState((prev) => {
      const index = prev.queue.findIndex((job) => job.jobId === jobId)
      if (index === -1) {
        return prev
      }

      const job = prev.queue[index]
      const canceledJob: QueueJob = {
        ...job,
        status: 'canceled',
        lastUpdated: Date.now(),
        cancelRequested: false,
      }

      cancelled = true

      return {
        ...prev,
        queue: [...prev.queue.slice(0, index), ...prev.queue.slice(index + 1)],
        completedJobs: [...prev.completedJobs, canceledJob],
      }
    })

    return cancelled
  }, [])

  const requestCancelCurrentJob = useCallback(async () => {
    const activeJob = state.currentJob
    if (!activeJob) {
      return false
    }

    if (activeJob.cancelRequested) {
      return true
    }

    setState((prev) => {
      if (!prev.currentJob || prev.currentJob.jobId !== activeJob.jobId) {
        return prev
      }

      return {
        ...prev,
        currentJob: {
          ...prev.currentJob,
          cancelRequested: true,
          lastUpdated: Date.now(),
        },
      }
    })

    appendLog('현재 작업 중단을 요청했습니다.')

    if (!isTauri()) {
      return true
    }

    try {
      await invoke('cancel_translation_job', { jobId: activeJob.jobId })
      return true
    } catch (error) {
      console.error('번역 작업 중단 요청에 실패했습니다.', error)
      appendLog('작업 중단 요청에 실패했습니다. 다시 시도해 주세요.', 'error')
      setState((prev) => {
        if (!prev.currentJob || prev.currentJob.jobId !== activeJob.jobId) {
          return prev
        }

        return {
          ...prev,
          currentJob: {
            ...prev.currentJob,
            cancelRequested: false,
            lastUpdated: Date.now(),
          },
        }
      })

      return false
    }
  }, [appendLog, state.currentJob])

  const toggleCurrentJobFileSelection = useCallback((path: string) => {
    setState((prev) => {
      if (!prev.currentJob || !prev.currentJob.files) {
        return prev
      }

      const index = prev.currentJob.files.findIndex((file) => file.path === path)
      if (index === -1) {
        return prev
      }

      const target = prev.currentJob.files[index]
      const nextSelected = !target.selected

      const nextFiles = [
        ...prev.currentJob.files.slice(0, index),
        { ...target, selected: nextSelected },
        ...prev.currentJob.files.slice(index + 1),
      ]

      const selectedEntries = nextFiles.filter((entry) => entry.selected)
      const selectedFiles = selectedEntries.map((entry) => entry.path)
      const sourceLanguageGuess = selectedEntries.length
        ? guessSourceLanguageFromFiles(selectedEntries)
        : DEFAULT_SOURCE_LANGUAGE

      return {
        ...prev,
        currentJob: {
          ...prev.currentJob,
          files: nextFiles,
          selectedFiles,
          sourceLanguageGuess,
          lastUpdated: Date.now(),
        },
      }
    })
  }, [])

  const loadFilesForCurrentJob = useCallback(async () => {
    const activeJob = state.currentJob
    if (!activeJob) {
      return
    }

    const jobId = activeJob.jobId
    const trimmedPath = activeJob.installPath.trim()

    if (!trimmedPath) {
      const message = '모드 설치 경로를 확인할 수 없어 작업이 실패했습니다.'
      let shouldFail = true
      setState((prev) => {
        if (!prev.currentJob || prev.currentJob.jobId !== jobId) {
          shouldFail = false
          return prev
        }

        return {
          ...prev,
          currentJob: {
            ...prev.currentJob,
            filesLoading: false,
            fileError: message,
            lastUpdated: Date.now(),
          },
        }
      })

      if (shouldFail) {
        markCurrentJobFailed(message)
      }
      return
    }

    if (!isTauri()) {
      const message = '파일 목록은 데스크톱 환경에서만 확인할 수 있습니다.'
      let shouldFail = true
      setState((prev) => {
        if (!prev.currentJob || prev.currentJob.jobId !== jobId) {
          shouldFail = false
          return prev
        }

        return {
          ...prev,
          currentJob: {
            ...prev.currentJob,
            filesLoading: false,
            fileError: message,
            lastUpdated: Date.now(),
          },
        }
      })

      if (shouldFail) {
        markCurrentJobFailed(message)
      }
      return
    }

    let shouldFetch = true
    setState((prev) => {
      if (!prev.currentJob || prev.currentJob.jobId !== jobId) {
        shouldFetch = false
        return prev
      }

      if (prev.currentJob.filesLoading) {
        shouldFetch = false
        return prev
      }

      if (prev.currentJob.files && !prev.currentJob.fileError) {
        shouldFetch = false
        return prev
      }

      return {
        ...prev,
        currentJob: {
          ...prev.currentJob,
          filesLoading: true,
          fileError: null,
        },
      }
    })

    if (!shouldFetch) {
      return
    }

    try {
      const listing = await invoke<ModFileListing>('list_mod_files', {
        modDirectory: trimmedPath,
      })

      setState((prev) => {
        if (!prev.currentJob || prev.currentJob.jobId !== jobId) {
          return prev
        }

        const existingSelection = prev.currentJob.selectedFiles
        const selectionSet = new Set(
          existingSelection.length
            ? existingSelection
            : listing.files
                .filter((entry) => entry.translatable && entry.auto_selected)
                .map((entry) => entry.path),
        )

        const files: JobFileEntry[] = listing.files.map((entry) => ({
          path: entry.path,
          translatable: entry.translatable,
          autoSelected: entry.auto_selected,
          languageHint: entry.language_hint,
          selected: selectionSet.has(entry.path),
        }))

        const selectedEntries = files.filter((entry) => entry.selected)
        const selectedFiles = selectedEntries.map((entry) => entry.path)
        const sourceLanguageGuess = selectedEntries.length
          ? guessSourceLanguageFromFiles(selectedEntries)
          : DEFAULT_SOURCE_LANGUAGE
        const targetLanguage = prev.currentJob.targetLanguage ?? DEFAULT_TARGET_LANGUAGE

        return {
          ...prev,
          currentJob: {
            ...prev.currentJob,
            files,
            filesLoading: false,
            fileError: null,
            selectedFiles,
            sourceLanguageGuess,
            targetLanguage,
            lastUpdated: Date.now(),
          },
        }
      })
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      let shouldFail = true
      setState((prev) => {
        if (!prev.currentJob || prev.currentJob.jobId !== jobId) {
          shouldFail = false
          return prev
        }

        return {
          ...prev,
          currentJob: {
            ...prev.currentJob,
            filesLoading: false,
            fileError: message,
            lastUpdated: Date.now(),
          },
        }
      })

      if (shouldFail) {
        markCurrentJobFailed('파일을 불러오지 못했습니다. 모드 경로나 설치 여부를 확인해 주세요.', {
          message,
        })
      }
    }
  }, [markCurrentJobFailed, state.currentJob])

  const startTranslationForCurrentJob = useCallback(
    async (options: StartTranslationOptions) => {
      if (!isTauri()) {
        throw new Error('번역 작업은 데스크톱 환경에서만 실행할 수 있습니다.')
      }

      const activeJob = state.currentJob
      if (!activeJob) {
        throw new Error('현재 실행 중인 작업이 없습니다.')
      }

      if (!options.selectedFiles.length) {
        throw new Error('번역할 파일을 하나 이상 선택해 주세요.')
      }

      if (!activeJob.providerId) {
        throw new Error('사용할 번역기를 설정한 뒤 다시 시도해 주세요.')
      }

      if (!activeJob.providerApiKey) {
        throw new Error('선택한 번역기의 API 키를 설정해 주세요.')
      }

      const targetLanguage = options.targetLanguage ?? activeJob.targetLanguage ?? DEFAULT_TARGET_LANGUAGE
      const sourceLanguage = options.sourceLanguageGuess ?? activeJob.sourceLanguageGuess ?? null

      const selectedSet = new Set(options.selectedFiles)
      let filesPayload = (activeJob.files ?? [])
        .filter((file) => selectedSet.has(file.path))
        .map((file) => ({ path: file.path }))

      if (!filesPayload.length) {
        filesPayload = options.selectedFiles.map((path) => ({ path }))
      }

      try {
        await invoke('start_translation_job', {
          jobId: activeJob.jobId,
          provider: activeJob.providerId,
          apiKey: activeJob.providerApiKey,
          sourceLang: sourceLanguage,
          targetLang: targetLanguage,
          files: filesPayload,
        })
      } catch (error) {
        throw error instanceof Error ? error : new Error(String(error))
      }

      setState((prev) => {
        if (!prev.currentJob || prev.currentJob.jobId !== activeJob.jobId) {
          return prev
        }

        return {
          ...prev,
          currentJob: {
            ...prev.currentJob,
            backendJobId: activeJob.jobId,
            message: '번역을 준비하는 중입니다.',
            status: 'running',
            progress: 0,
            cancelRequested: false,
            selectedFiles: [...options.selectedFiles],
            sourceLanguageGuess: sourceLanguage,
            targetLanguage,
            translatedCount: 0,
            totalCount: 0,
            logs: [...prev.currentJob.logs],
            lastUpdated: Date.now(),
          },
        }
      })
    },
    [state.currentJob],
  )

  useEffect(() => {
    if (!isTauri()) return undefined

    let cancelled = false
    let dispose: UnlistenFn | null = null

    listen<TranslationProgressEventPayload>('translation-progress', (event) => {
      if (cancelled) return
      const payload = event.payload
      if (!payload) return

      if (activeJobIdRef.current && activeJobIdRef.current !== payload.jobId) {
        return
      }

      const progress = clampProgress(Math.round(payload.progress))
      const trimmedLog = payload.log?.trim() ?? null
      const logLevel: JobLogLevel = payload.state === 'failed' ? 'error' : 'info'

      if (payload.state === 'completed') {
        const finalMessage = trimmedLog ?? '번역이 완료되었습니다.'
        markCurrentJobCompleted(finalMessage, {
          progress,
          message: finalMessage,
          translatedCount: payload.translatedCount,
          totalCount: payload.totalCount,
          cancelRequested: false,
        })
        return
      }

      if (payload.state === 'failed') {
        const combinedMessage = [payload.error, trimmedLog]
          .filter((value): value is string => Boolean(value))
          .join('\n') || '번역 중 오류가 발생했습니다.'
        markCurrentJobFailed(combinedMessage, {
          progress,
          message: combinedMessage,
          translatedCount: payload.translatedCount,
          totalCount: payload.totalCount,
          cancelRequested: false,
        })
        return
      }

      if (payload.state === 'canceled') {
        const cancelMessage = trimmedLog ?? '작업이 중단되었습니다.'
        markCurrentJobCanceled(cancelMessage, {
          progress,
          message: cancelMessage,
          translatedCount: payload.translatedCount,
          totalCount: payload.totalCount,
          cancelRequested: true,
        })
        return
      }

      setState((prev) => {
        if (!prev.currentJob || prev.currentJob.jobId !== payload.jobId) {
          return prev
        }

        const logEntry = trimmedLog ? createLogEntry(logLevel, trimmedLog) : null
        const logs = logEntry ? [...prev.currentJob.logs, logEntry] : prev.currentJob.logs

        return {
          ...prev,
          currentJob: {
            ...prev.currentJob,
            progress,
            status: payload.state,
            message: trimmedLog ?? prev.currentJob.message,
            translatedCount: payload.translatedCount,
            totalCount: payload.totalCount,
            logs,
            lastUpdated: Date.now(),
          },
        }
      })
    })
      .then((unlisten) => {
        if (cancelled) {
          unlisten()
        } else {
          dispose = unlisten
        }
      })
      .catch((error) => {
        console.error('translation-progress 이벤트 등록에 실패했습니다.', error)
      })

    return () => {
      cancelled = true
      if (dispose) {
        dispose()
      }
    }
  }, [markCurrentJobCanceled, markCurrentJobCompleted, markCurrentJobFailed])

  const value = useMemo<JobStoreValue>(
    () => ({
      currentJob: state.currentJob,
      queue: state.queue,
      completedJobs: state.completedJobs,
      enqueueJob,
      promoteNextJob,
      appendLog,
      markCurrentJobCompleted,
      markCurrentJobFailed,
      markCurrentJobCanceled,
      cancelQueuedJob,
      loadFilesForCurrentJob,
      toggleCurrentJobFileSelection,
      startTranslationForCurrentJob,
      requestCancelCurrentJob,
    }),
    [
      state.currentJob,
      state.queue,
      state.completedJobs,
      enqueueJob,
      promoteNextJob,
      appendLog,
      markCurrentJobCompleted,
      markCurrentJobFailed,
      markCurrentJobCanceled,
      cancelQueuedJob,
      loadFilesForCurrentJob,
      toggleCurrentJobFileSelection,
      startTranslationForCurrentJob,
      requestCancelCurrentJob,
    ],
  )

  return <JobStoreContext.Provider value={value}>{children}</JobStoreContext.Provider>
}

export function useJobStore() {
  const context = useContext(JobStoreContext)
  if (!context) {
    throw new Error('JobStore는 JobStoreProvider 내부에서만 사용할 수 있습니다.')
  }
  return context
}

