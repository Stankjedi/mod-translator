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
  TranslationFileErrorEntry,
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
  claude: '클로드',
  grok: '그록',
}

const createId = () =>
  typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : `job-${Date.now()}-${Math.random().toString(16).slice(2)}`

export type JobLogLevel = 'info' | 'warn' | 'error'

export interface JobLogEntry {
  ts: number
  text: string
  level: JobLogLevel
}

export interface JobFileEntry {
  path: string
  relativePath: string
  modInstallPath: string
  translatable: boolean
  autoSelected: boolean
  languageHint: string | null
  selected: boolean
}

export interface TranslationJob {
  id: string
  modId: string
  workshopId: string
  modName: string
  gameName: string
  installPath: string
  outputPath: string
  outputOverrideDir: string | null
  status: JobState
  providerId: ProviderId
  providerApiKey: string
  modelId: string
  progress: number
  translatedCount: number
  totalCount: number
  latestFileName: string | null
  latestFileSuccess: boolean | null
  cancelRequested: boolean
  logs: JobLogEntry[]
  files: JobFileEntry[] | null
  filesLoading: boolean
  fileListError: string | null
  fileErrors: TranslationFileErrorEntry[]
  selectedFiles: string[]
  sourceLanguageGuess: string
  targetLanguage: string
  createdAt: number
  completedAt?: number
}

interface JobStoreState {
  currentJob: TranslationJob | null
  queue: TranslationJob[]
  completedJobs: TranslationJob[]
}

interface FinalizeStats {
  progress: number
  translatedCount: number
  totalCount: number
  fileName: string | null
  fileSuccess: boolean | null
}

export interface EnqueueJobInput {
  modId: string
  workshopId?: string
  modName: string
  gameName: string
  installPath: string
}

export type EnqueueJobError =
  | 'duplicate-active'
  | 'duplicate-queued'
  | 'invalid-path'
  | 'missing-provider'
  | 'missing-api-key'
  | 'missing-model'

export interface EnqueueJobResult {
  job: TranslationJob
  promoted: boolean
  error: EnqueueJobError | null
}

export interface CancelRequestResult {
  success: boolean
  previousStatus: JobState | null
}

export interface StartTranslationOptions {
  selectedFiles: string[]
  sourceLanguageGuess: string | null
  targetLanguage?: string
}

interface JobStoreValue {
  currentJob: TranslationJob | null
  queue: TranslationJob[]
  completedJobs: TranslationJob[]
  enqueueJob: (input: EnqueueJobInput) => EnqueueJobResult
  appendLog: (message: string, level?: JobLogLevel) => void
  cancelQueuedJob: (jobId: string) => boolean
  loadFilesForCurrentJob: () => Promise<void>
  toggleCurrentJobFileSelection: (path: string) => void
  startTranslationForCurrentJob: (options: StartTranslationOptions) => Promise<void>
  requestCancelCurrentJob: () => Promise<CancelRequestResult>
  updateCurrentJobTargetLanguage: (value: string) => void
  updateCurrentJobOutputOverride: (value: string) => void
  dismissCurrentJob: () => void
}

const JobStoreContext = createContext<JobStoreValue | undefined>(undefined)

const clampProgress = (value: number) => {
  if (Number.isNaN(value)) {
    return 0
  }
  return Math.min(100, Math.max(0, value))
}

const createLogEntry = (level: JobLogLevel, text: string): JobLogEntry => ({
  ts: Date.now(),
  text,
  level,
})

const isTerminalStatus = (
  status: JobState,
): status is Extract<JobState, 'completed' | 'failed' | 'canceled' | 'partial_success'> =>
  status === 'completed' || status === 'failed' || status === 'canceled' || status === 'partial_success'

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

const applyFileErrorUpdates = (
  currentJob: TranslationJob,
  payload: TranslationProgressEventPayload,
): TranslationFileErrorEntry[] => {
  const prevList = currentJob.fileErrors ?? []
  const incoming = Array.isArray(payload.fileErrors) ? payload.fileErrors : []

  if (incoming.length === 0) {
    return prevList
  }

  const merged: TranslationFileErrorEntry[] = [...prevList]

  for (const entry of incoming) {
    if (!entry || typeof entry.filePath !== 'string' || typeof entry.message !== 'string') {
      continue
    }

    const normalizedPath = entry.filePath
    const normalizedMessage = entry.message
    const normalizedCode = entry.code

    const alreadyExists = merged.some(
      (item) =>
        item.filePath === normalizedPath &&
        item.message === normalizedMessage &&
        item.code === normalizedCode,
    )

    if (!alreadyExists) {
      const sanitized: TranslationFileErrorEntry = {
        filePath: normalizedPath,
        message: normalizedMessage,
      }

      if (typeof normalizedCode !== 'undefined') {
        sanitized.code = normalizedCode
      }

      merged.push(sanitized)
    }
  }

  return merged
}

const prepareJobForActivation = (job: TranslationJob): TranslationJob => ({
  ...job,
  status: 'pending',
  progress: 0,
  translatedCount: 0,
  totalCount: 0,
  latestFileName: null,
  latestFileSuccess: null,
  cancelRequested: false,
  logs: [],
  files: null,
  filesLoading: false,
  fileListError: null,
  fileErrors: [],
  selectedFiles: [...job.selectedFiles],
})

const createJob = (
  input: EnqueueJobInput,
  providerId: ProviderId,
  providerApiKey: string,
  modelId: string,
): TranslationJob => ({
  id: createId(),
  modId: input.modId,
  workshopId: input.workshopId ?? input.modId,
  modName: input.modName,
  gameName: input.gameName,
  installPath: input.installPath,
  outputPath: input.installPath,
  outputOverrideDir: null,
  status: 'pending',
  providerId,
  providerApiKey,
  modelId,
  progress: 0,
  translatedCount: 0,
  totalCount: 0,
  latestFileName: null,
  latestFileSuccess: null,
  cancelRequested: false,
  logs: [],
  files: null,
  filesLoading: false,
  fileListError: null,
  fileErrors: [],
  selectedFiles: [],
  sourceLanguageGuess: DEFAULT_SOURCE_LANGUAGE,
  targetLanguage: DEFAULT_TARGET_LANGUAGE,
  createdAt: Date.now(),
})

export function JobStoreProvider({ children }: { children: ReactNode }) {
  const { activeProviderId, apiKeys, providerModels } = useSettingsStore()
  const [state, setState] = useState<JobStoreState>({
    currentJob: null,
    queue: [],
    completedJobs: [],
  })
  const activeJobIdRef = useRef<string | null>(null)

  const providerApiKeyRef = useRef<string>('')

  useEffect(() => {
    activeJobIdRef.current = state.currentJob?.id ?? null
    providerApiKeyRef.current = state.currentJob?.providerApiKey ?? ''
  }, [state.currentJob?.id, state.currentJob?.providerApiKey])

  useEffect(() => {
    setState((prev) => {
      let nextCurrent = prev.currentJob
      let currentChanged = false

      if (nextCurrent && nextCurrent.status === 'pending') {
        const updatedModel = providerModels[nextCurrent.providerId].trim()
        if (updatedModel && updatedModel !== nextCurrent.modelId) {
          nextCurrent = { ...nextCurrent, modelId: updatedModel }
          currentChanged = true
        }
      }

      let queueChanged = false
      const nextQueue = prev.queue.map((job) => {
        if (job.status !== 'pending') {
          return job
        }

        const updatedModel = providerModels[job.providerId].trim()
        if (updatedModel && updatedModel !== job.modelId) {
          queueChanged = true
          return { ...job, modelId: updatedModel }
        }

        return job
      })

      if (!currentChanged && !queueChanged) {
        return prev
      }

      return {
        ...prev,
        currentJob: nextCurrent,
        queue: queueChanged ? nextQueue : prev.queue,
      }
    })
  }, [providerModels])

  const finalizeCurrentJob = useCallback(
    (
      nextState: Extract<JobState, 'completed' | 'failed' | 'canceled' | 'partial_success'>,
      finalLogEntry: JobLogEntry | null,
      finalStats: FinalizeStats,
      payload?: TranslationProgressEventPayload,
    ) => {
      setState((prev) => {
        if (!prev.currentJob) {
          return prev
        }

        const fileErrors =
          payload && prev.currentJob.id === payload.jobId
            ? applyFileErrorUpdates(prev.currentJob, payload)
            : prev.currentJob.fileErrors

        const finishedJob: TranslationJob = {
          ...prev.currentJob,
          status: nextState,
          progress: finalStats.progress,
          translatedCount: finalStats.translatedCount,
          totalCount: finalStats.totalCount,
          latestFileName: finalStats.fileName,
          latestFileSuccess: finalStats.fileSuccess,
          cancelRequested: false,
          logs: finalLogEntry
            ? [...prev.currentJob.logs, finalLogEntry]
            : prev.currentJob.logs,
          fileErrors: [...fileErrors],
          completedAt: Date.now(),
        }

        const existingIndex = prev.completedJobs.findIndex((job) => job.id === finishedJob.id)
        const completedJobs =
          existingIndex === -1
            ? [...prev.completedJobs, finishedJob]
            : prev.completedJobs.map((job, index) => (index === existingIndex ? finishedJob : job))

        activeJobIdRef.current = finishedJob.id

        return {
          ...prev,
          currentJob: finishedJob,
          completedJobs,
        }
      })
    },
    [],
  )

  const appendLog = useCallback((message: string, level: JobLogLevel = 'info') => {
    const trimmed = message.trim()
    if (!trimmed) {
      return
    }

    setState((prev) => {
      if (!prev.currentJob) {
        return prev
      }

      const logEntry = createLogEntry(level, trimmed)

      return {
        ...prev,
        currentJob: {
          ...prev.currentJob,
          logs: [...prev.currentJob.logs, logEntry],
        },
      }
    })
  }, [])

  const enqueueJob = useCallback(
    (input: EnqueueJobInput): EnqueueJobResult => {
      let outcome: EnqueueJobResult | null = null

      const providerId = activeProviderId
      if (!providerId) {
        const placeholderJob = createJob(input, 'gemini', '', '')
        outcome = { job: placeholderJob, promoted: false, error: 'missing-provider' }
        return outcome
      }

      const apiKeyRaw = apiKeys[providerId] ?? ''
      const apiKey = apiKeyRaw.trim()

      if (!apiKey) {
        const placeholderJob = createJob(input, providerId, '', '')
        outcome = { job: placeholderJob, promoted: false, error: 'missing-api-key' }
        return outcome
      }

      const selectedModel = providerModels[providerId]?.trim() ?? ''
      if (!selectedModel) {
        const placeholderJob = createJob(input, providerId, apiKey, '')
        outcome = { job: placeholderJob, promoted: false, error: 'missing-model' }
        return outcome
      }

      const baseJob = createJob(input, providerId, apiKey, selectedModel)

      setState((prev) => {
        if (prev.currentJob && prev.currentJob.modId === input.modId) {
          outcome = { job: prev.currentJob, promoted: false, error: 'duplicate-active' }
          return prev
        }

        const duplicateQueued = prev.queue.find((job) => job.modId === input.modId)
        if (duplicateQueued) {
          outcome = { job: duplicateQueued, promoted: false, error: 'duplicate-queued' }
          return prev
        }

        const trimmedPath = baseJob.installPath.trim()
        if (!trimmedPath) {
          const failureLog = createLogEntry(
            'error',
            '모드 설치 경로가 유효하지 않아 작업을 완료할 수 없습니다.',
          )

          const failedJob: TranslationJob = {
            ...baseJob,
            status: 'failed',
            logs: [failureLog],
            completedAt: Date.now(),
          }

          outcome = { job: failedJob, promoted: false, error: 'invalid-path' }

          return {
            ...prev,
            completedJobs: [...prev.completedJobs, failedJob],
          }
        }

        const normalizedJob: TranslationJob = {
          ...baseJob,
          installPath: trimmedPath,
          outputPath: baseJob.outputPath?.trim() || trimmedPath,
        }

        if (!prev.currentJob) {
          const activated = prepareJobForActivation(normalizedJob)
          outcome = { job: activated, promoted: true, error: null }
          activeJobIdRef.current = activated.id
          return {
            ...prev,
            currentJob: activated,
          }
        }

        outcome = { job: normalizedJob, promoted: false, error: null }
        return {
          ...prev,
          queue: [...prev.queue, normalizedJob],
        }
      })

      if (!outcome) {
        throw new Error('enqueueJob 결과를 결정하지 못했습니다.')
      }

      return outcome
    },
    [activeProviderId, apiKeys, providerModels],
  )

  const cancelQueuedJob = useCallback((jobId: string) => {
    let cancelled = false
    setState((prev) => {
      const index = prev.queue.findIndex((job) => job.id === jobId)
      if (index === -1) {
        return prev
      }

      const job = prev.queue[index]
      const canceledJob: TranslationJob = {
        ...job,
        status: 'canceled',
        cancelRequested: false,
        completedAt: Date.now(),
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

  const loadFilesForCurrentJob = useCallback(async () => {
    const jobId = state.currentJob?.id
    if (!jobId) {
      return
    }

    const trimmedPath = state.currentJob?.installPath.trim()
    if (!trimmedPath) {
      appendLog('설치 경로가 비어 있어 파일을 불러올 수 없습니다.', 'error')
      return
    }

    setState((prev) => {
      if (!prev.currentJob || prev.currentJob.id !== jobId) {
        return prev
      }

      if (prev.currentJob.filesLoading) {
        return prev
      }

      return {
        ...prev,
        currentJob: {
          ...prev.currentJob,
          filesLoading: true,
          fileListError: null,
        },
      }
    })

    if (!isTauri()) {
      return
    }

    try {
      const listing = await invoke<ModFileListing>('list_mod_files', {
        modDirectory: trimmedPath,
      })

      setState((prev) => {
        if (!prev.currentJob || prev.currentJob.id !== jobId) {
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
          relativePath: entry.path,
          modInstallPath: entry.mod_install_path,
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
            fileListError: null,
            selectedFiles,
            sourceLanguageGuess,
            targetLanguage,
          },
        }
      })
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      let shouldLog = true

      setState((prev) => {
        if (!prev.currentJob || prev.currentJob.id !== jobId) {
          shouldLog = false
          return prev
        }

        return {
          ...prev,
          currentJob: {
            ...prev.currentJob,
            filesLoading: false,
            fileListError: message,
          },
        }
      })

      if (shouldLog) {
        appendLog(
          `파일을 불러오지 못했습니다. 모드 경로나 설치 여부를 확인해 주세요.\n${message}`,
          'error',
        )
      }
    }
  }, [appendLog, state.currentJob])

  const toggleCurrentJobFileSelection = useCallback((path: string) => {
    setState((prev) => {
      if (
        !prev.currentJob ||
        prev.currentJob.status !== 'pending' ||
        !prev.currentJob.files
      ) {
        return prev
      }

      const files = prev.currentJob.files.map((entry) =>
        entry.path === path ? { ...entry, selected: !entry.selected } : entry,
      )
      const selectedFiles = files.filter((entry) => entry.selected).map((entry) => entry.path)
      const sourceLanguageGuess = selectedFiles.length
        ? guessSourceLanguageFromFiles(files.filter((entry) => entry.selected))
        : DEFAULT_SOURCE_LANGUAGE

      return {
        ...prev,
        currentJob: {
          ...prev.currentJob,
          files,
          selectedFiles,
          sourceLanguageGuess,
        },
      }
    })
  }, [])

  const startTranslationForCurrentJob = useCallback(
    async (options: StartTranslationOptions) => {
      if (!isTauri()) {
        throw new Error('번역 작업은 데스크톱 환경에서만 실행할 수 있습니다.')
      }

      const activeJob = state.currentJob
      if (!activeJob) {
        throw new Error('현재 실행 중인 작업이 없습니다.')
      }

      if (activeJob.status !== 'pending') {
        throw new Error('이미 실행 중인 작업입니다.')
      }

      if (!options.selectedFiles.length) {
        throw new Error('번역할 파일을 하나 이상 선택해 주세요.')
      }

      const modelId = activeJob.modelId.trim()
      if (!modelId) {
        throw new Error('번역에 사용할 모델을 선택해 주세요.')
      }

      const targetLanguage = options.targetLanguage ?? activeJob.targetLanguage
      const sourceLanguage = options.sourceLanguageGuess ?? activeJob.sourceLanguageGuess

      const selectedSet = new Set(options.selectedFiles)
      let filesPayload = (activeJob.files ?? [])
        .filter((file) => selectedSet.has(file.path))
        .map((file) => ({
          relativePath: file.relativePath,
          modInstallPath: file.modInstallPath,
        }))

      if (!filesPayload.length) {
        const fallbackRoot = activeJob.installPath.trim()
        filesPayload = options.selectedFiles.map((filePath) => ({
          relativePath: filePath,
          modInstallPath: fallbackRoot,
        }))
      }

      await invoke('start_translation_job', {
        jobId: activeJob.id,
        provider: activeJob.providerId,
        apiKey: activeJob.providerApiKey,
        modelId,
        sourceLang: sourceLanguage,
        targetLang: targetLanguage,
        files: filesPayload,
        outputOverrideDir: activeJob.outputOverrideDir,
      })

      setState((prev) => {
        if (!prev.currentJob || prev.currentJob.id !== activeJob.id) {
          return prev
        }

        return {
          ...prev,
          currentJob: {
            ...prev.currentJob,
            status: 'running',
            progress: 0,
            cancelRequested: false,
            selectedFiles: [...options.selectedFiles],
            sourceLanguageGuess: sourceLanguage ?? DEFAULT_SOURCE_LANGUAGE,
            targetLanguage,
            modelId,
            translatedCount: 0,
            totalCount: 0,
          },
        }
      })
    },
    [state.currentJob],
  )

  const requestCancelCurrentJob = useCallback(async () => {
    const activeJob = state.currentJob
    if (!activeJob) {
      return { success: false, previousStatus: null }
    }

    if (activeJob.cancelRequested) {
      return { success: true, previousStatus: activeJob.status }
    }

    if (activeJob.status !== 'pending' && activeJob.status !== 'running') {
      return { success: false, previousStatus: activeJob.status }
    }

    const previousStatus = activeJob.status

    setState((prev) => {
      if (!prev.currentJob || prev.currentJob.id !== activeJob.id) {
        return prev
      }

      return {
        ...prev,
        currentJob: {
          ...prev.currentJob,
          cancelRequested: true,
        },
      }
    })

    if (previousStatus === 'pending') {
      // wait for backend confirmation to append logs to avoid duplicates
    } else {
      appendLog('현재 작업 중단을 요청했습니다.')
    }

    if (!isTauri()) {
      if (previousStatus === 'pending') {
        const finalLog = createLogEntry('warn', 'User canceled while preparing.')
        finalizeCurrentJob(
          'canceled',
          finalLog,
          {
            progress: activeJob.progress,
            translatedCount: activeJob.translatedCount,
            totalCount: activeJob.totalCount,
            fileName: activeJob.latestFileName,
            fileSuccess: activeJob.latestFileSuccess,
          },
        )
      }

      return { success: true, previousStatus }
    }

    try {
      await invoke('cancel_translation_job', { jobId: activeJob.id })
      return { success: true, previousStatus }
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      appendLog(`작업 중단 요청에 실패했습니다: ${message}`, 'warn')

      setState((prev) => {
        if (!prev.currentJob || prev.currentJob.id !== activeJob.id) {
          return prev
        }

        return {
          ...prev,
          currentJob: {
            ...prev.currentJob,
            cancelRequested: false,
          },
        }
      })

      return { success: false, previousStatus }
    }
  }, [appendLog, finalizeCurrentJob, state.currentJob])

  const updateCurrentJobTargetLanguage = useCallback((value: string) => {
    setState((prev) => {
      if (!prev.currentJob || prev.currentJob.status !== 'pending') {
        return prev
      }

      return {
        ...prev,
        currentJob: {
          ...prev.currentJob,
          targetLanguage: value,
        },
      }
    })
  }, [])

  const updateCurrentJobOutputOverride = useCallback((value: string) => {
    setState((prev) => {
      if (!prev.currentJob || prev.currentJob.status !== 'pending') {
        return prev
      }

      return {
        ...prev,
        currentJob: {
          ...prev.currentJob,
          outputOverrideDir: value,
        },
      }
    })
  }, [])

  const handleTranslationProgressPayload = useCallback(
    (payload: TranslationProgressEventPayload) => {
      if (!payload || !activeJobIdRef.current || activeJobIdRef.current !== payload.jobId) {
        return
      }

      const status = payload.status ?? 'running'
      const trimmedLog = payload.log?.trim() ?? ''
      const progress = clampProgress(Math.round(payload.progressPct ?? 0))
      const fileName = payload.fileName ?? null
      const fileSuccess = payload.fileSuccess ?? null
      const translatedCount = payload.translatedCount ?? 0
      const totalCount = payload.totalCount ?? 0
      const lastWritten = payload.lastWritten ?? null

      if (status === 'running' || status === 'pending') {
        setState((prev) => {
          if (!prev.currentJob || prev.currentJob.id !== payload.jobId) {
            return prev
          }

          const providerKey = prev.currentJob.providerApiKey.trim()
          const containsSensitiveData =
            !!providerKey && providerKey.length > 0 && trimmedLog.includes(providerKey)

          const logs =
            trimmedLog && !containsSensitiveData
              ? [...prev.currentJob.logs, createLogEntry('info', trimmedLog)]
              : prev.currentJob.logs

          const updatedFileErrors = applyFileErrorUpdates(prev.currentJob, payload)
          const nextOutputPath =
            lastWritten?.outputAbsolutePath ?? prev.currentJob.outputPath
          const nextTranslatedCount =
            typeof payload.translatedCount === 'number'
              ? payload.translatedCount
              : prev.currentJob.translatedCount
          const nextTotalCount =
            typeof payload.totalCount === 'number'
              ? payload.totalCount
              : prev.currentJob.totalCount
          const nextCancelRequested =
            typeof payload.cancelRequested === 'boolean'
              ? payload.cancelRequested
              : prev.currentJob.cancelRequested

          return {
            ...prev,
            currentJob: {
              ...prev.currentJob,
              status,
              progress,
              translatedCount: nextTranslatedCount,
              totalCount: nextTotalCount,
              latestFileName: fileName,
              latestFileSuccess: fileSuccess,
              logs,
              fileErrors: updatedFileErrors,
              outputPath: nextOutputPath,
              cancelRequested: nextCancelRequested,
            },
          }
        })
        return
      }

      if (
        status === 'completed' ||
        status === 'failed' ||
        status === 'canceled' ||
        status === 'partial_success'
      ) {
        const level: JobLogLevel =
          status === 'failed'
            ? 'error'
            : status === 'canceled' || status === 'partial_success'
            ? 'warn'
            : 'info'
        const fallbackText =
          status === 'completed'
            ? '번역이 완료되었습니다.'
            : status === 'failed'
            ? '번역 중 오류가 발생했습니다.'
            : status === 'partial_success'
            ? '일부 파일에서 오류가 발생했습니다.'
            : '작업이 중단되었습니다.'
        const providerKey = providerApiKeyRef.current.trim()
        const containsSensitiveData =
          !!providerKey && providerKey.length > 0 && trimmedLog.includes(providerKey)
        const sanitizedLog = containsSensitiveData ? '' : trimmedLog
        const text = sanitizedLog || fallbackText
        const finalLogEntry = text ? createLogEntry(level, text) : null
        finalizeCurrentJob(
          status,
          finalLogEntry,
          {
            progress,
            translatedCount,
            totalCount,
            fileName,
            fileSuccess,
          },
          payload,
        )
      }
    },
    [finalizeCurrentJob],
  )

  const dismissCurrentJob = useCallback(() => {
    setState((prev) => {
      if (!prev.currentJob || !isTerminalStatus(prev.currentJob.status)) {
        return prev
      }

      const finishedJob = prev.currentJob
      const queue = [...prev.queue]
      const nextJob = queue.shift() ?? null
      const nextCurrent = nextJob ? prepareJobForActivation(nextJob) : null

      const existingIndex = prev.completedJobs.findIndex((job) => job.id === finishedJob.id)
      const completedJobs =
        existingIndex === -1
          ? [...prev.completedJobs, finishedJob]
          : prev.completedJobs.map((job, index) => (index === existingIndex ? finishedJob : job))

      activeJobIdRef.current = nextCurrent?.id ?? null

      return {
        ...prev,
        currentJob: nextCurrent,
        queue,
        completedJobs,
      }
    })
  }, [])

  useEffect(() => {
    if (!isTauri()) {
      return
    }

    let unlistenFn: UnlistenFn | null = null
    let disposed = false

    const setup = async () => {
      try {
        unlistenFn = await listen<TranslationProgressEventPayload>(
          'translation-progress',
          (event) => {
            if (disposed) {
              return
            }
            handleTranslationProgressPayload(event.payload)
          },
        )
      } catch (error) {
        console.error('translation-progress 이벤트 등록에 실패했습니다.', error)
      }
    }

    setup()

    return () => {
      disposed = true
      if (unlistenFn) {
        unlistenFn()
      }
    }
  }, [handleTranslationProgressPayload])

  const value = useMemo<JobStoreValue>(
    () => ({
      currentJob: state.currentJob,
      queue: state.queue,
      completedJobs: state.completedJobs,
      enqueueJob,
      appendLog,
      cancelQueuedJob,
      loadFilesForCurrentJob,
      toggleCurrentJobFileSelection,
      startTranslationForCurrentJob,
      requestCancelCurrentJob,
      updateCurrentJobTargetLanguage,
      updateCurrentJobOutputOverride,
      dismissCurrentJob,
    }),
    [
      state.currentJob,
      state.queue,
      state.completedJobs,
      enqueueJob,
      appendLog,
      cancelQueuedJob,
      loadFilesForCurrentJob,
      toggleCurrentJobFileSelection,
      startTranslationForCurrentJob,
      requestCancelCurrentJob,
      updateCurrentJobTargetLanguage,
      updateCurrentJobOutputOverride,
      dismissCurrentJob,
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

export function getDefaultJobRoute(hasActiveJob: boolean, queueLength: number): '/progress' | '/mods' {
  return hasActiveJob || queueLength > 0 ? '/progress' : '/mods'
}

export const providerLabelFor = (provider: ProviderId) => PROVIDER_LABELS[provider] ?? provider.toUpperCase()
