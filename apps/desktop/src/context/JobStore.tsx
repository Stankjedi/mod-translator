/* eslint-disable react-refresh/only-export-components */
import type { ReactNode } from 'react'
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import type { UnlistenFn } from '@tauri-apps/api/event'
import type {
  JobStatusUpdatedEvent,
  JobState,
  QueueSnapshot,
  QualityGateSnapshot,
  RateLimiterSnapshot,
  ModFileListing,
  TranslationJobRequest,
  TranslationJobStatus,
  TranslatorKind,
} from '../types/core'
import { getStoredProviderAuth } from '../storage/apiKeyStorage'

const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

const DEFAULT_TRANSLATOR: TranslatorKind = 'gpt'
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
  preview: string | null
  translator: string | null
  queueSnapshot: QueueSnapshot | null
  rateLimiter: RateLimiterSnapshot | null
  qualityGates: QualityGateSnapshot | null
  pipeline: TranslationJobStatus['pipeline'] | null
  selectedFiles: string[]
  sourceLanguageGuess: string | null
  targetLanguage: string | null
  lastUpdated: number
  files: JobFileEntry[] | null
  filesLoading: boolean
  fileError: string | null
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
  translator?: TranslatorKind
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
  cancelQueuedJob: (jobId: string) => boolean
  loadFilesForCurrentJob: () => Promise<void>
  toggleCurrentJobFileSelection: (path: string) => void
  startTranslationForCurrentJob: (options: StartTranslationOptions) => Promise<TranslationJobStatus>
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
  preview: null,
  translator: null,
  queueSnapshot: null,
  rateLimiter: null,
  qualityGates: null,
  pipeline: null,
  logs: [],
  selectedFiles: [],
  sourceLanguageGuess: null,
  targetLanguage: job.targetLanguage ?? DEFAULT_TARGET_LANGUAGE,
  lastUpdated: Date.now(),
  files: null,
  filesLoading: false,
  fileError: null,
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
  preview: null,
  translator: null,
  queueSnapshot: null,
  rateLimiter: null,
  qualityGates: null,
  pipeline: null,
  selectedFiles: [],
  sourceLanguageGuess: null,
  targetLanguage: DEFAULT_TARGET_LANGUAGE,
  lastUpdated: Date.now(),
  files: null,
  filesLoading: false,
  fileError: null,
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
  const [state, setState] = useState<JobStoreState>({
    currentJob: null,
    queue: [],
    completedJobs: [],
  })

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

  const markCurrentJobCompleted = useCallback((message?: string | null, patch?: Partial<QueueJob>) => {
    const entry = message ? createLogEntry('info', message) : null
    setState((prev) => {
      if (!prev.currentJob) return prev

      const logs = entry ? [...prev.currentJob.logs, entry] : prev.currentJob.logs
      const completedJob: QueueJob = {
        ...prev.currentJob,
        ...patch,
        status: 'completed',
        progress: 100,
        logs,
        lastUpdated: Date.now(),
      }

      const baseState: JobStoreState = {
        ...prev,
        currentJob: null,
        completedJobs: [...prev.completedJobs, completedJob],
      }

      return promoteNextJobState(baseState)
    })
  }, [])

  const markCurrentJobFailed = useCallback((message?: string | null, patch?: Partial<QueueJob>) => {
    const entry = message ? createLogEntry('error', message) : null
    setState((prev) => {
      if (!prev.currentJob) return prev

      const logs = entry ? [...prev.currentJob.logs, entry] : prev.currentJob.logs
      const failedJob: QueueJob = {
        ...prev.currentJob,
        ...patch,
        status: 'failed',
        progress: patch?.progress ?? prev.currentJob.progress,
        logs,
        lastUpdated: Date.now(),
      }

      const baseState: JobStoreState = {
        ...prev,
        currentJob: null,
        completedJobs: [...prev.completedJobs, failedJob],
      }

      return promoteNextJobState(baseState)
    })
  }, [])

  const enqueueJob = useCallback((input: EnqueueJobInput): EnqueueJobResult => {
    let result: EnqueueJobResult | null = null

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

      const job = createJob(input)
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
  }, [])

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

      const translator = options.translator ?? DEFAULT_TRANSLATOR
      const targetLanguage = options.targetLanguage ?? DEFAULT_TARGET_LANGUAGE

      const request: TranslationJobRequest = {
        mod_id: activeJob.modId,
        mod_name: activeJob.modName,
        translator,
        source_language_guess: options.sourceLanguageGuess ?? null,
        target_language: targetLanguage,
        selected_files: options.selectedFiles,
        provider_auth: getStoredProviderAuth(),
      }

      const status = await invoke<TranslationJobStatus>('start_translation_job', {
        request,
      })

      const progress = clampProgress(Math.round(status.progress * 100))

      if (status.state === 'completed') {
        markCurrentJobCompleted(status.message ?? null, {
          backendJobId: status.job_id,
          translator: status.translator,
          message: status.message,
          preview: status.preview,
          queueSnapshot: status.queue,
          rateLimiter: status.rate_limiter,
          qualityGates: status.quality_gates,
          pipeline: status.pipeline,
          progress,
          targetLanguage,
        })
        return status
      }

      if (status.state === 'failed') {
        markCurrentJobFailed(status.message ?? null, {
          backendJobId: status.job_id,
          translator: status.translator,
          message: status.message,
          preview: status.preview,
          queueSnapshot: status.queue,
          rateLimiter: status.rate_limiter,
          qualityGates: status.quality_gates,
          pipeline: status.pipeline,
          progress,
          targetLanguage,
        })
        return status
      }

      const trimmedMessage = status.message?.trim()
      const logEntry = trimmedMessage ? createLogEntry('info', trimmedMessage) : null

      setState((prev) => {
        if (!prev.currentJob || prev.currentJob.jobId !== activeJob.jobId) {
          return prev
        }

        const logs = logEntry ? [...prev.currentJob.logs, logEntry] : prev.currentJob.logs

        return {
          ...prev,
          currentJob: {
            ...prev.currentJob,
            backendJobId: status.job_id,
            translator: status.translator,
            message: status.message,
            preview: status.preview,
            queueSnapshot: status.queue,
            rateLimiter: status.rate_limiter,
            qualityGates: status.quality_gates,
            pipeline: status.pipeline,
            progress,
            status: status.state,
            selectedFiles: [...options.selectedFiles],
            sourceLanguageGuess: options.sourceLanguageGuess ?? null,
            targetLanguage,
            logs,
            lastUpdated: Date.now(),
          },
        }
      })

      return status
    },
    [markCurrentJobCompleted, markCurrentJobFailed, state.currentJob],
  )

  useEffect(() => {
    if (!isTauri()) return undefined

    let cancelled = false
    let dispose: UnlistenFn | null = null

    listen<JobStatusUpdatedEvent>('job-status-updated', (event) => {
      if (cancelled) return
      const payload = event.payload
      if (!payload) return

      const status = payload.status
      const progress = clampProgress(Math.round(status.progress * 100))
      const trimmedMessage = status.message?.trim()
      const logLevel: JobLogLevel = status.state === 'failed' ? 'error' : 'info'
      const logEntry = trimmedMessage ? createLogEntry(logLevel, trimmedMessage) : null

      if (status.state === 'completed') {
        markCurrentJobCompleted(status.message ?? null, {
          backendJobId: payload.job_id,
          translator: status.translator,
          message: status.message,
          preview: status.preview,
          queueSnapshot: status.queue,
          rateLimiter: status.rate_limiter,
          qualityGates: status.quality_gates,
          pipeline: status.pipeline,
          progress,
        })
        return
      }

      if (status.state === 'failed') {
        markCurrentJobFailed(status.message ?? null, {
          backendJobId: payload.job_id,
          translator: status.translator,
          message: status.message,
          preview: status.preview,
          queueSnapshot: status.queue,
          rateLimiter: status.rate_limiter,
          qualityGates: status.quality_gates,
          pipeline: status.pipeline,
          progress,
        })
        return
      }

      setState((prev) => {
        if (!prev.currentJob) return prev

        const sameMod = prev.currentJob.modId === payload.mod_id
        const sameBackendJob =
          !prev.currentJob.backendJobId || prev.currentJob.backendJobId === payload.job_id

        if (!sameMod || !sameBackendJob) {
          return prev
        }

        const logs = logEntry ? [...prev.currentJob.logs, logEntry] : prev.currentJob.logs

        return {
          ...prev,
          currentJob: {
            ...prev.currentJob,
            backendJobId: payload.job_id ?? prev.currentJob.backendJobId,
            translator: status.translator ?? prev.currentJob.translator,
            message: status.message,
            preview: status.preview,
            queueSnapshot: status.queue,
            rateLimiter: status.rate_limiter,
            qualityGates: status.quality_gates,
            pipeline: status.pipeline,
            progress,
            status: status.state,
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
        console.error('job-status-updated 이벤트 등록에 실패했습니다.', error)
      })

    return () => {
      cancelled = true
      if (dispose) {
        dispose()
      }
    }
  }, [markCurrentJobCompleted, markCurrentJobFailed])

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
      cancelQueuedJob,
      loadFilesForCurrentJob,
      toggleCurrentJobFileSelection,
      startTranslationForCurrentJob,
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
      cancelQueuedJob,
      loadFilesForCurrentJob,
      toggleCurrentJobFileSelection,
      startTranslationForCurrentJob,
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

