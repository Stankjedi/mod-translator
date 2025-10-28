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
import type {
  ModSummary,
  TranslationJobRequest,
  TranslationJobStatus,
  TranslatorKind,
} from '../types/core'

interface JobEntry {
  jobId: string
  status: TranslationJobStatus
  modId: string
}

interface JobContextValue {
  jobsByMod: Record<string, JobEntry>
  startJob: (mod: ModSummary, options: StartJobOptions) => Promise<TranslationJobStatus>
  refreshJob: (modId: string) => Promise<void>
}

const JobContext = createContext<JobContextValue | undefined>(undefined)

const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export interface StartJobOptions {
  selectedFiles: string[]
  sourceLanguageGuess: string
  targetLanguage?: string
  translator?: TranslatorKind
}

function resolveTranslator(options: StartJobOptions) {
  return options.translator ?? ('gpt' as TranslatorKind)
}

function resolveTargetLanguage(options: StartJobOptions) {
  return options.targetLanguage ?? 'ko'
}

export function JobProvider({ children }: { children: ReactNode }) {
  const [jobsByMod, setJobsByMod] = useState<Record<string, JobEntry>>({})

  const startJob = useCallback(
    async (mod: ModSummary, options: StartJobOptions) => {
      if (!isTauri()) {
        throw new Error('번역 작업은 데스크톱(Tauri) 환경에서만 실행할 수 있습니다.')
      }

      if (!options.selectedFiles.length) {
        throw new Error('번역할 파일을 하나 이상 선택해야 합니다.')
      }

      const existing = jobsByMod[mod.id]
      if (existing) {
        const state = existing.status.state
        if (state === 'queued' || state === 'running') {
          return existing.status
        }
      }

      const request: TranslationJobRequest = {
        mod_id: mod.id,
        mod_name: mod.name,
        translator: resolveTranslator(options),
        source_language_guess: options.sourceLanguageGuess,
        target_language: resolveTargetLanguage(options),
        selected_files: options.selectedFiles,
      }

      const status = await invoke<TranslationJobStatus>('start_translation_job', {
        request,
      })

      setJobsByMod((prev) => ({
        ...prev,
        [mod.id]: { jobId: status.job_id, status, modId: mod.id },
      }))

      return status
    },
    [jobsByMod],
  )

  const refreshJob = useCallback(
    async (modId: string) => {
      if (!isTauri()) return
      const entry = jobsByMod[modId]
      if (!entry) return

      try {
        const status = await invoke<TranslationJobStatus>('get_translation_job_status', {
          job_id: entry.jobId,
        })
        setJobsByMod((prev) => {
          const current = prev[modId]
          if (!current) return prev
          return {
            ...prev,
            [modId]: { ...current, status },
          }
        })
      } catch (err) {
        console.error(err)
      }
    },
    [jobsByMod],
  )

  const activeJobs = useMemo(
    () =>
      Object.entries(jobsByMod).filter(([, entry]) =>
        entry.status.state === 'queued' || entry.status.state === 'running',
      ),
    [jobsByMod],
  )

  useEffect(() => {
    if (!activeJobs.length || !isTauri()) return

    const interval = window.setInterval(() => {
      activeJobs.forEach(([modId, entry]) => {
        invoke<TranslationJobStatus>('get_translation_job_status', {
          job_id: entry.jobId,
        })
          .then((status) => {
            setJobsByMod((prev) => {
              const current = prev[modId]
              if (!current) return prev
              return {
                ...prev,
                [modId]: { ...current, status },
              }
            })
          })
          .catch((err) => {
            console.error(err)
          })
      })
    }, 2000)

    return () => {
      window.clearInterval(interval)
    }
  }, [activeJobs])

  const value = useMemo(
    () => ({
      jobsByMod,
      startJob,
      refreshJob,
    }),
    [jobsByMod, startJob, refreshJob],
  )

  return <JobContext.Provider value={value}>{children}</JobContext.Provider>
}

export function useJobContext() {
  const context = useContext(JobContext)
  if (!context) {
    throw new Error('JobContext는 JobProvider 내에서만 사용할 수 있습니다.')
  }
  return context
}
