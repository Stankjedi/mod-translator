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
  JobStatusUpdatedEvent,
  JobState,
  QueueSnapshot,
  QualityGateSnapshot,
  RateLimiterSnapshot,
  TranslationJobRequest,
  TranslationJobStatus,
  TranslatorKind,
} from '../types/core'
import { getStoredProviderAuth } from '../storage/apiKeyStorage'

const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

const DEFAULT_TRANSLATOR: TranslatorKind = 'gpt'
const DEFAULT_TARGET_LANGUAGE = 'ko'

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
  setCurrentJobSelection: (
    selectedFiles: string[],
    sourceLanguageGuess: string | null,
    targetLanguage?: string | null,
  ) => void
  startTranslationForCurrentJob: (options: StartTranslationOptions) => Promise<TranslationJobStatus>
}

const JobStoreContext = createContext<JobStoreValue | undefined>(undefined)

const createLogEntry = (level: JobLogLevel, message: string): JobLogEntry => ({
  id: createId(),
  level,
  message,
  timestamp: Date.now(),
})

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
  selectedFiles: job.selectedFiles ?? [],
  sourceLanguageGuess: job.sourceLanguageGuess ?? null,
  targetLanguage: job.targetLanguage ?? null,
  lastUpdated: Date.now(),
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
})

const arraysEqual = (a: string[], b: string[]) => {
  if (a.length !== b.length) return false
  for (let index = 0; index < a.length; index += 1) {
    if (a[index] !== b[index]) return false
  }
  return true
}

export function JobStoreProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<JobStoreState>({
    currentJob: null,
    queue: [],
    completedJobs: [],
  })

  const stateRef = useRef(state)
  useEffect(() => {
    stateRef.current = state
  }, [state])

  const promoteNextJob = useCallback(() => {
    setState((prev) => {
      if (!prev.queue.length) {
        if (!prev.currentJob) return prev
        return {
          ...prev,
          currentJob: null,
          queue: [],
        }
      }

      const [nextJob, ...rest] = prev.queue
      return {
        currentJob: prepareJobForActivation(nextJob),
        queue: rest,
        completedJobs: prev.completedJobs,
      }
    })
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

  const markCurrentJobCompleted = useCallback(
    (message?: string | null, patch?: Partial<QueueJob>) => {
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
        return {
          currentJob: null,
          queue: prev.queue,
          completedJobs: [...prev.completedJobs, completedJob],
        }
      })
      promoteNextJob()
    },
    [promoteNextJob],
  )

  const markCurrentJobFailed = useCallback(
    (message?: string | null, patch?: Partial<QueueJob>) => {
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
        return {
          currentJob: null,
          queue: prev.queue,
          completedJobs: [...prev.completedJobs, failedJob],
        }
      })
      promoteNextJob()
    },
    [promoteNextJob],
  )

  const setCurrentJobSelection = useCallback(
    (selectedFiles: string[], sourceLanguageGuess: string | null, targetLanguage?: string | null) => {
      setState((prev) => {
        if (!prev.currentJob) return prev
        const normalizedGuess = sourceLanguageGuess ?? null
        const normalizedTarget = targetLanguage ?? null
        if (
          arraysEqual(prev.currentJob.selectedFiles, selectedFiles) &&
          prev.currentJob.sourceLanguageGuess === normalizedGuess &&
          prev.currentJob.targetLanguage === normalizedTarget
        ) {
          return prev
        }

        return {
          ...prev,
          currentJob: {
            ...prev.currentJob,
            selectedFiles,
            sourceLanguageGuess: normalizedGuess,
            targetLanguage: normalizedTarget,
            lastUpdated: Date.now(),
          },
        }
      })
    },
    [],
  )

  const enqueueJob = useCallback(
    (input: EnqueueJobInput): EnqueueJobResult => {
      const snapshot = stateRef.current

      if (snapshot.currentJob && snapshot.currentJob.modId === input.modId) {
        return { job: snapshot.currentJob, promoted: false, error: 'duplicate-active' }
      }

      const duplicateQueued = snapshot.queue.find((job) => job.modId === input.modId)
      if (duplicateQueued) {
        return { job: duplicateQueued, promoted: false, error: 'duplicate-queued' }
      }

      const job = createJob(input)
      if (!job.installPath || !job.installPath.trim()) {
        const failureLog = createLogEntry(
          'error',
          '모드 설치 경로가 유효하지 않아 작업을 완료할 수 없습니다.',
        )
        const failedJob: QueueJob = {
          ...job,
          status: 'failed',
          logs: [failureLog],
        }
        setState((prev) => ({
          currentJob: prev.currentJob,
          queue: prev.queue,
          completedJobs: [...prev.completedJobs, failedJob],
        }))
        return { job: failedJob, promoted: false, error: 'invalid-path' }
      }

      let promoted = false
      setState((prev) => {
        const queue = [...prev.queue, job]
        if (!prev.currentJob) {
          const [nextJob, ...rest] = queue
          promoted = true
          return {
            currentJob: prepareJobForActivation(nextJob),
            queue: rest,
            completedJobs: prev.completedJobs,
          }
        }

        return {
          ...prev,
          queue,
        }
      })

      return { job, promoted, error: null }
    },
    [],
  )

  const startTranslationForCurrentJob = useCallback(
    async (options: StartTranslationOptions) => {
      if (!isTauri()) {
        throw new Error('번역 작업은 데스크톱 환경에서만 실행할 수 있습니다.')
      }

      const snapshot = stateRef.current
      const activeJob = snapshot.currentJob
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

      const logLevel: JobLogLevel = status.state === 'failed' ? 'error' : 'info'
      const logEntry = status.message && status.message.trim()
        ? createLogEntry(logLevel, status.message)
        : null

      setState((prev) => {
        if (!prev.currentJob) return prev
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
            progress: Math.round(status.progress * 100),
            status: status.state,
            selectedFiles: [...options.selectedFiles],
            sourceLanguageGuess: options.sourceLanguageGuess ?? null,
            targetLanguage,
            logs,
            lastUpdated: Date.now(),
          },
        }
      })

      if (status.state === 'completed') {
        markCurrentJobCompleted(null, {
          backendJobId: status.job_id,
          translator: status.translator,
          message: status.message,
          preview: status.preview,
          queueSnapshot: status.queue,
          rateLimiter: status.rate_limiter,
          qualityGates: status.quality_gates,
          pipeline: status.pipeline,
          progress: Math.round(status.progress * 100),
        })
      } else if (status.state === 'failed') {
        markCurrentJobFailed(null, {
          backendJobId: status.job_id,
          translator: status.translator,
          message: status.message,
          preview: status.preview,
          queueSnapshot: status.queue,
          rateLimiter: status.rate_limiter,
          qualityGates: status.quality_gates,
          pipeline: status.pipeline,
          progress: Math.round(status.progress * 100),
        })
      }

      return status
    },
    [markCurrentJobCompleted, markCurrentJobFailed],
  )

  useEffect(() => {
    if (!isTauri()) return undefined

    let cancelled = false
    let dispose: UnlistenFn | null = null

    listen<JobStatusUpdatedEvent>('job-status-updated', (event) => {
      if (cancelled) return
      const payload = event.payload
      if (!payload) return

      const snapshot = stateRef.current
      const activeJob = snapshot.currentJob
      if (!activeJob) return
      if (
        activeJob.modId !== payload.mod_id &&
        activeJob.backendJobId &&
        activeJob.backendJobId !== payload.job_id
      ) {
        return
      }

      const status = payload.status
      const progress = Math.round(status.progress * 100)
      const logLevel: JobLogLevel = status.state === 'failed' ? 'error' : 'info'
      const logEntry = status.message && status.message.trim()
        ? createLogEntry(logLevel, status.message)
        : null

      if (status.state === 'completed') {
        markCurrentJobCompleted(null, {
          backendJobId: payload.job_id,
          translator: status.translator,
          message: status.message,
          preview: status.preview,
          queueSnapshot: status.queue,
          rateLimiter: status.rate_limiter,
          qualityGates: status.quality_gates,
          pipeline: status.pipeline,
          progress,
          logs: logEntry ? [...activeJob.logs, logEntry] : activeJob.logs,
        })
        return
      }

      if (status.state === 'failed') {
        markCurrentJobFailed(null, {
          backendJobId: payload.job_id,
          translator: status.translator,
          message: status.message,
          preview: status.preview,
          queueSnapshot: status.queue,
          rateLimiter: status.rate_limiter,
          qualityGates: status.quality_gates,
          pipeline: status.pipeline,
          progress,
          logs: logEntry ? [...activeJob.logs, logEntry] : activeJob.logs,
        })
        return
      }

      setState((prev) => {
        if (!prev.currentJob) return prev
        const logs = logEntry ? [...prev.currentJob.logs, logEntry] : prev.currentJob.logs
        return {
          ...prev,
          currentJob: {
            ...prev.currentJob,
            backendJobId: payload.job_id,
            translator: status.translator,
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
      setCurrentJobSelection,
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
      setCurrentJobSelection,
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

