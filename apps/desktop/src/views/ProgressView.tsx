import { useMemo, useState } from 'react'
import { useLibraryContext } from '../context/LibraryContext'
import { useJobContext } from '../context/JobContext'
import type { JobState, ModSummary } from '../types/core'

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

function ProgressView() {
  const { libraries } = useLibraryContext()
  const { jobsByMod, startJob, refreshJob } = useJobContext()
  const [pendingMods, setPendingMods] = useState<Record<string, boolean>>({})

  const trackedMods = useMemo(
    () => libraries.flatMap((library) => library.mods),
    [libraries],
  )

  const setPending = (modId: string, value: boolean) => {
    setPendingMods((prev) => {
      if (value) {
        return { ...prev, [modId]: true }
      }
      const { [modId]: _removed, ...rest } = prev
      return rest
    })
  }

  const handleStart = async (mod: ModSummary) => {
    setPending(mod.id, true)
    try {
      await startJob(mod)
    } catch (err) {
      console.error(err)
    } finally {
      setPending(mod.id, false)
    }
  }

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
      <header>
        <h2 className="text-xl font-semibold text-white">번역 진행 상황</h2>
        <p className="text-sm text-slate-400">
          감지된 모드별로 번역 작업을 예약하고 Rust 백엔드의 작업 큐에서 보고된 상태를 실시간으로 확인하세요.
        </p>
      </header>

      {trackedMods.length ? (
        <div className="space-y-4">
          {trackedMods.map((mod) => {
            const jobEntry = jobsByMod[mod.id]
            const status = jobEntry?.status
            const progressValue = Math.round((status?.progress ?? 0) * 100)
            const clampedProgress = Math.max(0, Math.min(100, progressValue))
            const stateClass = status ? stateClasses[status.state] : 'bg-slate-800 text-slate-300'
            const progressBarClass = status ? progressClasses[status.state] : 'bg-slate-700'
            const isPending = Boolean(pendingMods[mod.id])
            const isRunning = status ? status.state === 'queued' || status.state === 'running' : false

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

                <div className="mt-4 flex flex-wrap gap-2">
                  <button
                    type="button"
                    onClick={() => handleStart(mod)}
                    disabled={isPending || isRunning}
                    className="rounded-lg bg-brand-600 px-4 py-2 text-sm font-semibold text-white shadow shadow-brand-600/40 transition hover:bg-brand-500 disabled:cursor-not-allowed disabled:opacity-60"
                  >
                    {isRunning ? '진행 중' : '번역 시작'}
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
              </article>
            )
          })}
        </div>
      ) : (
        <div className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-10 text-center">
          <p className="text-base font-semibold text-white">등록된 모드가 없습니다.</p>
          <p className="mt-2 text-sm text-slate-400">
            라이브러리를 스캔하여 모드를 감지하면 이곳에 작업 대기열이 표시됩니다.
          </p>
        </div>
      )}
    </div>
  )
}

export default ProgressView
