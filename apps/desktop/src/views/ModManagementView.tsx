import { useCallback, useMemo, useState } from 'react'
import type { MouseEvent } from 'react'
import { useNavigate } from 'react-router-dom'
import { useLibraryContext } from '../context/LibraryContext'
import { useJobStore } from '../context/JobStore'
import Chip, { type ChipTone } from '../ui/Chip'
import type { JobState, LibraryStatus } from '../types/core'

const languageLabels: Record<string, string> = {
  en: '영어',
  ko: '한국어',
  ja: '일본어',
  zh: '중국어',
  ru: '러시아어',
  es: '스페인어',
  fr: '프랑스어',
  de: '독일어',
}

function resolveLanguage(code: string) {
  const lowered = code.toLowerCase()
  return languageLabels[lowered] ?? lowered.toUpperCase()
}

const ALL_GAMES = 'ALL'

interface ModRow {
  id: string
  name: string
  game: string
  normalizedGame: string
  languages: string[]
  status: LibraryStatus
  warnings: string[]
  workshopRoot: string | null
  libraryPath: string
  directory: string
  lastUpdated: string
}

function ModManagementView() {
  const { libraries, isScanning, scanLibrary, steamPath } = useLibraryContext()
  const [selectedGame, setSelectedGame] = useState<string>(ALL_GAMES)
  const [searchQuery, setSearchQuery] = useState('')
  const { currentJob, queue, enqueueJob, cancelQueuedJob } = useJobStore()
  const [actionMessage, setActionMessage] = useState<{
    type: 'success' | 'error'
    text: string
  } | null>(null)
  const navigate = useNavigate()

  const allMods = useMemo<ModRow[]>(() => {
    const deduped = new Map<string, ModRow>()

    libraries.forEach((library) => {
      library.mods.forEach((mod) => {
        const game = mod.game ?? ''
        const normalizedGame = game.replace(/\s+/g, '')
        const modKey = mod.id
        if (!deduped.has(modKey)) {
          deduped.set(modKey, {
            id: mod.id,
            name: mod.name,
            game,
            normalizedGame,
            languages: mod.installed_languages,
            status: library.status,
            warnings: mod.warnings,
            workshopRoot: library.workshop_root,
            libraryPath: library.path,
            directory: mod.directory,
            lastUpdated: mod.last_updated.iso_date,
          })
        }
      })
    })

    return Array.from(deduped.values())
  }, [libraries])

  const gameOptions = useMemo(() => {
    const normalizedToRaw = new Map<string, string>()

    allMods.forEach((mod) => {
      if (!normalizedToRaw.has(mod.normalizedGame)) {
        normalizedToRaw.set(mod.normalizedGame, mod.game)
      }
    })

    const options: Array<{ value: string; label: string }> = [
      { value: ALL_GAMES, label: '모든 게임' },
    ]

    const emptyKeyValue = normalizedToRaw.get('')
    if (emptyKeyValue !== undefined) {
      options.push({ value: emptyKeyValue, label: emptyKeyValue })
    }

    const sorted = Array.from(normalizedToRaw.entries())
      .filter(([key]) => key.length > 0)
      .sort((a, b) => a[0].localeCompare(b[0], 'ko'))
      .map(([, rawValue]) => ({ value: rawValue, label: rawValue }))

    return [...options, ...sorted]
  }, [allMods])

  const selectedGameLabel = useMemo(() => {
    const match = gameOptions.find((option) => option.value === selectedGame)
    return match ? match.label : selectedGame
  }, [gameOptions, selectedGame])

  const visibleMods = useMemo(() => {
    const normalizedSelectedGame =
      selectedGame === ALL_GAMES ? '' : selectedGame.replace(/\s+/g, '')

    const filteredByGame =
      selectedGame === ALL_GAMES
        ? allMods
        : allMods.filter((mod) => mod.normalizedGame === normalizedSelectedGame)

    const normalizedQuery = searchQuery.trim().toLowerCase()
    if (!normalizedQuery) {
      return filteredByGame
    }

    return filteredByGame.filter((mod) => {
      const nameMatch = mod.name.toLowerCase().includes(normalizedQuery)
      const idMatch = mod.id.toLowerCase().includes(normalizedQuery)
      return nameMatch || idMatch
    })
  }, [allMods, selectedGame, searchQuery])

  const jobLookup = useMemo(() => {
    const map = new Map<
      string,
      {
        status: JobState
        position?: number
        progress?: number
        jobId?: string
        cancelRequested?: boolean
      }
    >()
    if (currentJob) {
      map.set(currentJob.modId, {
        status: currentJob.status,
        progress: Math.round(currentJob.progress),
        jobId: currentJob.id,
        cancelRequested: currentJob.cancelRequested,
      })
    }

    queue.forEach((job, index) => {
      map.set(job.modId, {
        status: job.status,
        position: index + 1,
        jobId: job.id,
        cancelRequested: job.cancelRequested,
      })
    })

    return map
  }, [currentJob, queue])

  const handleSelectMod = useCallback(
    (mod: ModRow) => {
      setActionMessage(null)

      const isCurrent = currentJob?.modId === mod.id
      const queuedJob = queue.find((job) => job.modId === mod.id)

      if (isCurrent || queuedJob) {
        navigate('/progress')
        return
      }

      try {
        const result = enqueueJob({
          modId: mod.id,
          workshopId: mod.id,
          modName: mod.name,
          gameName: mod.game,
          installPath: mod.directory,
        })

        if (result.error === 'invalid-path') {
          setActionMessage({
            type: 'error',
            text: '설치 경로를 찾을 수 없어 작업이 실패로 기록되었습니다. 라이브러리 경로를 확인하세요.',
          })
          return
        }

        if (result.error === 'missing-provider' || result.error === 'missing-api-key') {
          setActionMessage({
            type: 'error',
            text: '선택한 번역기의 API 키가 설정되지 않았습니다. 설정 탭에서 API 키를 입력한 뒤 다시 시도해 주세요.',
          })
          navigate('/settings')
          return
        }

        if (result.error === 'missing-model') {
          setActionMessage({
            type: 'error',
            text: '사용할 모델이 지정되지 않았습니다. 설정 탭에서 제공자별 모델을 선택한 뒤 다시 시도해 주세요.',
          })
          navigate('/settings')
          return
        }

        navigate('/progress')
      } catch (error) {
        console.error('모드 예약 중 오류가 발생했습니다.', error)
        setActionMessage({
          type: 'error',
          text: '작업을 예약하는 중 문제가 발생했습니다. 다시 시도해 주세요.',
        })
      }
    },
    [currentJob, enqueueJob, navigate, queue],
  )

  const handleCancelQueuedJob = useCallback(
    (event: MouseEvent<HTMLButtonElement>, jobId: string) => {
      event.stopPropagation()
      cancelQueuedJob(jobId)
    },
    [cancelQueuedJob],
  )

  const hasAnyMods = allMods.length > 0
  const hasSearchQuery = searchQuery.trim().length > 0

  return (
    <div className="space-y-6">
      <header className="flex flex-col gap-3 lg:flex-row lg:items-center lg:justify-between">
        <div>
          <h2 className="text-xl font-semibold text-white">설치된 모드</h2>
          <p className="text-sm text-slate-400">
            실제 스캔 결과를 기반으로 워크샵 콘텐츠를 표시합니다. 게임 필터를 활용하여 특정 타이틀의 모드만 빠르게
            확인할 수 있습니다.
          </p>
        </div>
        <div className="flex flex-col gap-2 sm:flex-row sm:items-center">
          <input
            type="search"
            value={searchQuery}
            onChange={(event) => setSearchQuery(event.target.value)}
            placeholder="모드 검색"
            className="w-full rounded-xl border border-slate-800 bg-slate-900/80 px-3 py-2 text-sm text-slate-200 focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500 sm:w-64"
          />
          <select
            value={selectedGame}
            onChange={(event) => setSelectedGame(event.target.value)}
            className="w-full rounded-xl border border-slate-800 bg-slate-900/80 px-3 py-2 text-sm text-slate-200 focus:border-brand-500 focus:outline-none focus:ring-1 focus:ring-brand-500 sm:w-56"
          >
            {gameOptions.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))}
          </select>
          <button
            type="button"
            onClick={() => {
              scanLibrary(steamPath ?? undefined).catch((error) => {
                console.error('모드 관리 화면에서 라이브러리 스캔 실패', error)
              })
            }}
            disabled={isScanning}
            className="inline-flex items-center justify-center rounded-xl bg-brand-600 px-4 py-2 text-sm font-semibold text-white shadow shadow-brand-600/40 transition hover:bg-brand-500 disabled:cursor-not-allowed disabled:opacity-60"
          >
            {isScanning ? '스캔 중...' : '라이브러리 스캔'}
          </button>
        </div>
      </header>

      {actionMessage && (
        <div
          className={`rounded-2xl border px-4 py-3 text-sm ${
            actionMessage.type === 'success'
              ? 'border-emerald-500/40 bg-emerald-500/10 text-emerald-200'
              : 'border-rose-500/40 bg-rose-500/10 text-rose-200'
          }`}
        >
          {actionMessage.text}
        </div>
      )}

      <div className="overflow-hidden rounded-2xl border border-slate-800/60 bg-slate-900/60">
        {visibleMods.length ? (
          <table className="min-w-full divide-y divide-slate-800 text-sm text-slate-200">
            <thead className="bg-slate-900/80 text-xs uppercase tracking-wider text-slate-400">
              <tr>
                <th scope="col" className="px-4 py-3 text-left">
                  모드 이름
                </th>
                <th scope="col" className="px-4 py-3 text-left">
                  게임
                </th>
                <th scope="col" className="px-4 py-3 text-left">
                  지원 언어
                </th>
                <th scope="col" className="px-4 py-3 text-left">
                  라이브러리 경로
                </th>
                <th scope="col" className="px-4 py-3 text-left">
                  경고 / 참고
                </th>
                <th scope="col" className="px-4 py-3 text-left">
                  작업 제어
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/60">
              {visibleMods.map((mod, index) => {
                const jobInfo = jobLookup.get(mod.id)
                const isCurrent = currentJob?.modId === mod.id
                const isRunning = isCurrent && jobInfo?.status === 'running'
                const isPending = isCurrent && jobInfo?.status === 'pending'
                const isQueued = !isCurrent && jobInfo?.status === 'pending'
                const isCancelRequested = Boolean(isCurrent && jobInfo?.cancelRequested)
                const queuePosition = jobInfo?.position
                const statusNote = isRunning
                  ? `실행 중 · ${jobInfo?.progress ?? 0}%`
                  : isPending
                    ? '시작 대기 중'
                  : isQueued
                    ? queuePosition
                      ? `대기열 ${queuePosition}번`
                      : '대기열 정보 없음'
                    : '대기열에 없음'
                const jobId = jobInfo?.jobId
                const badgeLabel = isCancelRequested
                  ? '중단 요청됨'
                  : isRunning
                    ? '진행 중'
                    : isPending
                      ? '준비 중'
                      : isQueued
                        ? `대기 중${queuePosition ? ` ${queuePosition}번` : ''}`
                        : '예약 가능'
                const badgeTone: ChipTone = isCancelRequested
                  ? 'warning'
                  : isRunning
                    ? 'running'
                    : isPending
                      ? 'idle'
                      : isQueued
                        ? 'idle'
                        : 'primary'
                const note = isCancelRequested
                  ? `사용자 중단 요청 진행 중 · ${jobInfo?.progress ?? 0}%`
                  : statusNote

                return (
                  <tr
                    key={mod.id}
                    onClick={() => handleSelectMod(mod)}
                    className="cursor-pointer align-top transition hover:bg-slate-800/40"
                  >
                    <td className="px-4 py-4 font-medium text-white">
                      <div className="flex items-baseline gap-2">
                        <span className="text-xs font-semibold text-slate-500">{index + 1}.</span>
                        <span>{mod.name}</span>
                      </div>
                      <p className="mt-1 text-xs text-slate-400">마지막 업데이트: {mod.lastUpdated}</p>
                      <p className="mt-1 text-[11px] text-slate-500">워크샵 ID: {mod.id}</p>
                    </td>
                    <td className="px-4 py-4 text-slate-300">
                      <div className="font-medium text-white">{mod.game}</div>
                      <p className="mt-1 text-xs text-slate-500">
                        상태: {mod.status === 'healthy' ? '정상' : '경로 확인 필요'}
                      </p>
                    </td>
                    <td className="px-4 py-4 text-slate-300">
                      <div className="flex flex-wrap gap-2">
                        {mod.languages.map((language) => (
                          <span key={language} className="rounded-full bg-slate-800 px-2 py-1 text-xs text-slate-300">
                            {resolveLanguage(language)}
                          </span>
                        ))}
                      </div>
                    </td>
                    <td className="px-4 py-4 text-xs text-slate-400">
                      <p className="break-all font-medium text-white">{mod.libraryPath}</p>
                      {mod.workshopRoot && (
                        <p className="mt-1 text-[11px] text-slate-500">워크샵 루트: {mod.workshopRoot}</p>
                      )}
                    </td>
                    <td className="px-4 py-4 text-xs text-slate-300">
                      {mod.warnings.length ? (
                        <ul className="space-y-1">
                          {mod.warnings.map((warning) => (
                            <li key={warning} className="rounded bg-slate-800/80 px-2 py-1 text-[11px] text-amber-300">
                              {warning}
                            </li>
                          ))}
                        </ul>
                      ) : (
                        <p className="text-slate-500">경고 없음</p>
                      )}
                    </td>
                    <td className="px-4 py-4 text-xs text-slate-300">
                      <div className="flex flex-col gap-2">
                        <Chip label={badgeLabel} tone={badgeTone} />
                        <p className="text-[11px] text-slate-500">{note}</p>
                        {isQueued && jobId && (
                          <button
                            type="button"
                            onClick={(event) => handleCancelQueuedJob(event, jobId)}
                            className="inline-flex w-fit items-center justify-center rounded-full border border-rose-500/40 px-3 py-1 text-[11px] font-semibold text-rose-200 transition hover:border-rose-400 hover:text-rose-100"
                          >
                            취소
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                )
              })}
            </tbody>
          </table>
        ) : (
          <div className="flex flex-col items-center justify-center gap-3 px-6 py-16 text-center text-slate-400">
            <p className="text-base font-semibold text-white">
              {hasAnyMods
                ? hasSearchQuery
                  ? '검색 조건과 일치하는 모드를 찾지 못했습니다.'
                  : `${selectedGame === ALL_GAMES ? '모든 게임' : selectedGameLabel}에 해당하는 모드를 찾지 못했습니다.`
                : '표시할 모드가 없습니다.'}
            </p>
            <p className="text-sm">
              {hasAnyMods
                ? hasSearchQuery
                  ? '다른 키워드로 검색하거나 검색어를 지워 전체 목록을 확인해 보세요.'
                  : '다른 게임을 선택하거나 라이브러리 스캔을 다시 실행해 보세요.'
                : 'Steam을 실행하여 워크샵 콘텐츠를 다운로드한 뒤, 상단의 스캔 버튼을 눌러 목록을 새로고침하세요.'}
            </p>
          </div>
        )}
      </div>

      <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6">
        <h3 className="text-lg font-semibold text-white">다음 단계</h3>
        <ul className="mt-3 space-y-2 text-sm text-slate-300">
          <li>진행 상황 탭에서 번역 작업을 예약하고 상태를 모니터링하세요.</li>
          <li>Steam이 새 모드를 설치하면 라이브러리 스캔을 다시 실행해 메타데이터를 갱신하세요.</li>
          <li>경고가 포함된 모드는 내보내기 전에 검증 도구를 실행해 이상 여부를 확인하세요.</li>
        </ul>
      </section>
    </div>
  )
}

export default ModManagementView
