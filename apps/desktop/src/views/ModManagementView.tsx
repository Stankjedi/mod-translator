import { useMemo } from 'react'
import { useLibraryContext } from '../context/LibraryContext'

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

function ModManagementView() {
  const { libraries, isScanning, scanLibrary, steamPath } = useLibraryContext()

  const modRows = useMemo(
    () =>
      libraries.flatMap((library) =>
        library.mods.map((mod) => ({
          id: mod.id,
          name: mod.name,
          game: mod.game,
          languages: mod.installed_languages,
          status: library.status,
          policy: mod.policy,
          warnings: mod.warnings,
          workshopRoot: library.workshop_root,
          libraryPath: library.path,
          lastUpdated: mod.last_updated.iso_date,
        })),
      ),
    [libraries],
  )

  return (
    <div className="space-y-6">
      <header className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h2 className="text-xl font-semibold text-white">설치된 모드</h2>
          <p className="text-sm text-slate-400">
            실제 스캔 결과를 기반으로 워크샵 콘텐츠를 표시합니다. 라이브러리 스캔을 다시 실행하면 목록이 즉시 갱신됩니다.
          </p>
        </div>
        <button
          type="button"
          onClick={() => scanLibrary(steamPath?.path ?? undefined)}
          disabled={isScanning}
          className="inline-flex items-center justify-center rounded-xl bg-brand-600 px-4 py-2 text-sm font-semibold text-white shadow shadow-brand-600/40 transition hover:bg-brand-500 disabled:cursor-not-allowed disabled:opacity-60"
        >
          {isScanning ? '스캔 중...' : '라이브러리 스캔'}
        </button>
      </header>

      <div className="overflow-hidden rounded-2xl border border-slate-800/60 bg-slate-900/60">
        {modRows.length ? (
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
                  정책 / 메모
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-slate-800/60">
              {modRows.map((mod) => (
                <tr key={mod.id} className="hover:bg-slate-800/60">
                  <td className="px-4 py-4 font-medium text-white">
                    <div>{mod.name}</div>
                    <p className="mt-1 text-xs text-slate-400">마지막 업데이트: {mod.lastUpdated}</p>
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
                    <p className="font-medium text-white break-all">{mod.libraryPath}</p>
                    {mod.workshopRoot && (
                      <p className="mt-1 text-[11px] text-slate-500">워크샵 루트: {mod.workshopRoot}</p>
                    )}
                  </td>
                  <td className="px-4 py-4 text-xs text-slate-300">
                    <p className="font-medium text-white">
                      {mod.policy.notes[0] ?? '추가 정책 메모가 없습니다.'}
                    </p>
                    {mod.policy.eula_reference && (
                      <p className="mt-1 text-[11px] text-slate-500">{mod.policy.eula_reference}</p>
                    )}
                    <ul className="mt-2 space-y-1">
                      {mod.warnings.map((warning) => (
                        <li key={warning} className="rounded bg-slate-800/80 px-2 py-1 text-[11px] text-amber-300">
                          {warning}
                        </li>
                      ))}
                    </ul>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        ) : (
          <div className="flex flex-col items-center justify-center gap-3 px-6 py-16 text-center text-slate-400">
            <p className="text-base font-semibold text-white">표시할 모드가 없습니다.</p>
            <p className="text-sm">
              Steam을 실행하여 워크샵 콘텐츠를 다운로드한 뒤, 상단의 스캔 버튼을 눌러 목록을 새로고침하세요.
            </p>
          </div>
        )}
      </div>

      <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6">
        <h3 className="text-lg font-semibold text-white">다음 단계</h3>
        <ul className="mt-3 space-y-2 text-sm text-slate-300">
          <li>게임별 정책 프로필을 확인한 뒤 번역 작업을 예약하세요.</li>
          <li>Steam이 새 모드를 설치하면 라이브러리 스캔을 다시 실행해 메타데이터를 갱신하세요.</li>
          <li>경고가 포함된 모드는 내보내기 전에 검증 도구를 실행해 이상 여부를 확인하세요.</li>
        </ul>
      </section>
    </div>
  )
}

export default ModManagementView
