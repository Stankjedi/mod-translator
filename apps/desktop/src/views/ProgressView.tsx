import { useMemo } from 'react'
import { useLibraryContext } from '../context/LibraryContext'

function ProgressView() {
  const { libraries } = useLibraryContext()

  const trackedMods = useMemo(
    () =>
      libraries.flatMap((library) =>
        library.mods.map((mod) => ({
          id: mod.id,
          name: mod.name,
          game: mod.game,
          warnings: mod.warnings,
          policy: mod.policy,
        })),
      ),
    [libraries],
  )

  return (
    <div className="space-y-6">
      <header>
        <h2 className="text-xl font-semibold text-white">번역 진행 상황</h2>
        <p className="text-sm text-slate-400">
          Rust 코어의 작업 큐와 연동하여 실제 번역 상태를 표시할 준비가 되어 있습니다. 현재는 실행 중인 작업이
          없어 라이브러리 스캔 결과만 요약합니다.
        </p>
      </header>

      {trackedMods.length ? (
        <div className="space-y-4">
          {trackedMods.map((mod) => (
            <article
              key={mod.id}
              className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30"
            >
              <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
                <div>
                  <h3 className="text-lg font-semibold text-white">{mod.name}</h3>
                  <p className="text-xs uppercase tracking-wider text-slate-500">{mod.id}</p>
                  <p className="mt-1 text-sm text-slate-400">대상 게임: {mod.game}</p>
                </div>
                <span className="rounded-full bg-slate-800 px-3 py-1 text-xs font-semibold text-slate-300">
                  작업 대기
                </span>
              </div>
              <p className="mt-3 text-xs text-slate-400">
                아직 번역 작업이 실행되지 않았습니다. 설정 탭에서 AI 공급자를 활성화한 뒤 작업 큐와 연동하면 진행률이
                여기에 표시됩니다.
              </p>
              <div className="mt-3 grid gap-2 text-[11px] text-slate-400 sm:grid-cols-2">
                <div className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-3">
                  <p className="font-semibold text-slate-200">재배포 정책</p>
                  <ul className="mt-1 space-y-1">
                    {mod.policy.notes.map((note) => (
                      <li key={note}>{note}</li>
                    ))}
                  </ul>
                </div>
                <div className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-3">
                  <p className="font-semibold text-slate-200">주의 사항</p>
                  {mod.warnings.length ? (
                    <ul className="mt-1 space-y-1">
                      {mod.warnings.map((warning) => (
                        <li key={warning}>{warning}</li>
                      ))}
                    </ul>
                  ) : (
                    <p className="mt-1">추가 경고가 없습니다.</p>
                  )}
                </div>
              </div>
            </article>
          ))}
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
