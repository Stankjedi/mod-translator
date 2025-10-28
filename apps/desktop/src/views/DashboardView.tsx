import { useCallback, useMemo } from 'react'
import { useNavigate } from 'react-router-dom'
import { useLibraryContext } from '../context/LibraryContext'
import { useJobStore } from '../context/JobStore'
import Chip from '../components/Chip'

const pipelineStages = [
  '워크샵 압축 해제',
  '파일 형식 식별 및 텍스트 자산 분류',
  'JSON/INI/XML/RESX 리소스 파싱',
  '플레이스홀더 고정 후 번역 실행',
  '플레이스홀더와 마크업 검증',
  '리소스 재패키징 또는 패치 생성',
]

function DashboardView() {
  const { libraries, isScanning, scanLibrary, steamPath } = useLibraryContext()
  const { currentJob, queue, completedJobs } = useJobStore()
  const navigate = useNavigate()

  const totalLibraries = libraries.length
  const healthyLibraries = useMemo(
    () => libraries.filter((library) => library.status === 'healthy').length,
    [libraries],
  )
  const totalMods = useMemo(
    () => libraries.reduce((sum, library) => sum + library.mods.length, 0),
    [libraries],
  )
  const totalWarnings = useMemo(
    () =>
      libraries.reduce(
        (sum, library) =>
          sum + library.mods.reduce((warningSum, mod) => warningSum + mod.warnings.length, 0),
        0,
      ),
    [libraries],
  )

  const allMods = useMemo(
    () => libraries.flatMap((library) => library.mods),
    [libraries],
  )
  const firstDetectedMod = useMemo(() => allMods[0] ?? null, [allMods])

  const firstNote = useMemo(() => {
    const note = libraries.find((library) => library.notes.length > 0)?.notes[0]
    if (note) return note
    if (isScanning) return '워크샵 콘텐츠를 찾는 중입니다.'
    if (!totalLibraries) return '스팀 경로가 확인되면 자동으로 스캔이 실행됩니다.'
    return '라이브러리가 정상적으로 감지되었습니다.'
  }, [libraries, isScanning, totalLibraries])

  const availableWorkshops = useMemo(
    () => libraries.filter((library) => library.workshop_root).length,
    [libraries],
  )

  const jobHighlight = useMemo(() => {
    const runningCount = currentJob ? 1 : 0
    const queuedCount = queue.length
    const completedCount = completedJobs.filter((job) => job.status === 'completed').length
    const failedCount = completedJobs.filter((job) => job.status === 'failed').length

    if (!runningCount && !queuedCount) {
      if (!completedJobs.length) {
        return '예약된 번역 작업이 없습니다. 모드 관리 탭에서 새 작업을 추가해 보세요.'
      }

      const pieces = [`최근 완료 ${completedCount}건`]
      if (failedCount) {
        pieces.push(`실패 ${failedCount}건`)
      }
      return pieces.join(' · ')
    }

    const summaryPieces = [`실행 중 ${runningCount}건`, `대기 ${queuedCount}건`, `완료 ${completedCount}건`]
    if (failedCount) {
      summaryPieces.push(`실패 ${failedCount}건`)
    }
    return summaryPieces.join(' · ')
  }, [completedJobs, currentJob, queue.length])

  const metrics = [
    {
      title: '감지된 라이브러리',
      value: isScanning ? '스캔 중' : `${totalLibraries}개`,
      hint: `정상 경로 ${healthyLibraries}개`,
    },
    {
      title: '발견된 모드',
      value: `${totalMods}개`,
      hint: availableWorkshops
        ? `워크샵 루트 ${availableWorkshops}개`
        : '워크샵 루트를 찾지 못했습니다',
    },
    {
      title: '주의 항목',
      value: `${totalWarnings}건`,
      hint: totalWarnings
        ? '경고를 확인하고 필요한 작업을 진행하세요.'
        : '추가 조치가 필요한 경고가 없습니다.',
    },
  ]

  const highlights = [
    {
      title: '라이브러리 스캔 결과',
      description: firstNote,
    },
    {
      title: '워크샵 경로',
      description: availableWorkshops
        ? '워크샵 콘텐츠가 연결된 라이브러리를 찾았습니다.'
        : '워크샵 경로를 찾지 못했습니다. Steam을 한 번 실행한 뒤 다시 시도하세요.',
    },
    {
      title: '번역 작업 현황',
      description: jobHighlight,
    },
  ]

  const gameSummaries = useMemo(() => {
    const map = new Map<string, { game: string; modCount: number; warningCount: number }>()
    libraries.forEach((library) => {
      library.mods.forEach((mod) => {
        const existing = map.get(mod.game)
        if (existing) {
          existing.modCount += 1
          existing.warningCount += mod.warnings.length
        } else {
          map.set(mod.game, {
            game: mod.game,
            modCount: 1,
            warningCount: mod.warnings.length,
          })
        }
      })
    })

    return Array.from(map.values())
      .sort((a, b) => b.modCount - a.modCount)
      .slice(0, 4)
  }, [libraries])

  const handleJobAction = useCallback(() => {
    if (currentJob || queue.length) {
      navigate('/progress')
      return
    }

    navigate('/mods')
  }, [currentJob, navigate, queue.length])

  const quickActions = [
    {
      label: isScanning ? '스캔 중...' : '라이브러리 다시 스캔',
      description: 'Steam 경로의 libraryfolders.vdf를 다시 읽어서 모드 목록을 갱신합니다.',
      onClick: async () => {
        try {
          await scanLibrary(steamPath ?? undefined)
        } catch (error) {
          console.error('대시보드에서 라이브러리 스캔 실패', error)
        }
      },
      disabled: isScanning,
    },
    {
      label: currentJob
        ? `${currentJob.modName} 작업 보기`
        : queue.length
          ? '대기 중 작업 확인'
          : '번역 작업 예약',
      description: currentJob
        ? `${currentJob.modName} · 진행률 ${currentJob.progress}%`
        : queue.length
          ? `대기열에 ${queue.length}건의 작업이 준비되어 있습니다.`
          : firstDetectedMod
            ? `${firstDetectedMod.name} 모드를 선택해 번역을 예약하세요.`
            : '작업을 예약하려면 먼저 라이브러리를 스캔해 모드를 확인하세요.',
      onClick: handleJobAction,
      disabled: false,
    },
    {
      label: '품질 가드 설정',
      description: '추후 번역 품질 검증 도구와 연동될 예정입니다.',
      disabled: true,
    },
  ]

  return (
    <div className="space-y-10">
      <section>
        <h2 className="text-xl font-semibold text-white">오늘의 요약</h2>
        <p className="text-sm text-slate-400">
          정책 동의 상태와 라이브러리 스캔 결과, 워크샵 경고를 한 자리에서 확인할 수 있습니다.
        </p>
        <div className="mt-6 grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
          {metrics.map((metric) => (
            <div
              key={metric.title}
              className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-5 shadow-inner shadow-black/30"
            >
              <div className="text-sm font-medium text-slate-400">{metric.title}</div>
              <div className="mt-2 text-3xl font-semibold text-white">{metric.value}</div>
              <div className="mt-2 text-xs text-slate-500">{metric.hint}</div>
            </div>
          ))}
        </div>
      </section>

      <section className="grid gap-6 lg:grid-cols-2">
        <div className="space-y-4 rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6">
          <h3 className="text-lg font-semibold text-white">운영 하이라이트</h3>
          <ul className="space-y-3 text-sm text-slate-300">
            {highlights.map((item) => (
              <li key={item.title} className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-4">
                <div className="font-medium text-white">{item.title}</div>
                <p className="mt-1 text-slate-400">{item.description}</p>
              </li>
            ))}
          </ul>
        </div>
        <div className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6">
          <h3 className="text-lg font-semibold text-white">빠른 작업</h3>
          <ul className="mt-4 space-y-3 text-sm text-slate-300">
            {quickActions.map((action) => (
              <li
                key={action.label}
                className="flex items-start justify-between rounded-lg border border-slate-800/60 bg-slate-900/60 p-4"
              >
                <div>
                  <div className="text-sm font-semibold text-white">{action.label}</div>
                  <p className="mt-1 text-xs text-slate-400">{action.description}</p>
                </div>
                {action.onClick ? (
                  <Chip
                    label="실행"
                    variant="action"
                    tone={action.disabled ? 'idle' : 'primary'}
                    onClick={action.onClick}
                    disabled={action.disabled}
                  />
                ) : (
                  <Chip label="준비 중" variant="action" tone="idle" />
                )}
              </li>
            ))}
          </ul>
        </div>
      </section>

      <section className="grid gap-6 lg:grid-cols-2">
        <div className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6">
          <h3 className="text-lg font-semibold text-white">게임별 모드 분포</h3>
          {gameSummaries.length ? (
            <ul className="mt-4 space-y-3 text-sm text-slate-300">
              {gameSummaries.map((summary) => (
                <li key={summary.game} className="rounded-xl border border-slate-800/60 bg-slate-900/60 p-4">
                  <div className="text-sm font-semibold text-white">{summary.game}</div>
                  <p className="mt-1 text-xs text-slate-400">
                    감지된 모드 {summary.modCount}개 · 경고 {summary.warningCount}건
                  </p>
                </li>
              ))}
            </ul>
          ) : (
            <p className="mt-4 text-sm text-slate-400">스캔된 모드가 없어 분포 정보를 표시할 수 없습니다.</p>
          )}
        </div>
        <div className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6">
          <h3 className="text-lg font-semibold text-white">파이프라인 스냅샷</h3>
          <ol className="mt-4 space-y-2 text-sm text-slate-300">
            {pipelineStages.map((stage) => (
              <li key={stage} className="flex items-start gap-3">
                <span className="mt-1 h-2 w-2 rounded-full bg-brand-500" aria-hidden />
                <span>{stage}</span>
              </li>
            ))}
          </ol>
        </div>
      </section>
    </div>
  )
}

export default DashboardView
