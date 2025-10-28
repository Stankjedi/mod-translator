import { useMemo, useState } from 'react'
import { NavLink, Route, Routes } from 'react-router-dom'
import { LibraryProvider, useLibraryContext } from './context/LibraryContext'
import { JobProvider } from './context/JobContext'
import DashboardView from './views/DashboardView'
import ModManagementView from './views/ModManagementView'
import ProgressView from './views/ProgressView'
import SettingsView from './views/SettingsView'
import {
  getPolicyAcknowledged,
  setPolicyAcknowledged as persistPolicyAcknowledged,
} from './storage/policyStorage'

const navigation = [
  { to: '/', label: '대시보드', description: '최근 상태와 진행 현황 요약' },
  { to: '/mods', label: '모드 관리', description: '설치된 워크샵 콘텐츠 살펴보기' },
  { to: '/progress', label: '진행 상황', description: '번역 파이프라인 모니터링' },
  { to: '/settings', label: '설정', description: '번역기 환경과 스캔 옵션 구성' },
]

function AppShell() {
  const { policyBanner, error } = useLibraryContext()
  const [policyAcknowledged, setPolicyAcknowledgedState] = useState(() => getPolicyAcknowledged())

  const showPolicyBanner = useMemo(() => {
    if (!policyBanner) return false
    if (!policyBanner.requires_acknowledgement) return false
    return !policyAcknowledged
  }, [policyBanner, policyAcknowledged])

  return (
    <div className="flex min-h-screen bg-slate-950 text-slate-100">
      <aside className="hidden w-72 flex-col border-r border-slate-800/60 bg-slate-900/60 backdrop-blur md:flex">
        <div className="px-6 py-8">
          <p className="text-sm font-semibold uppercase tracking-widest text-slate-400">모드 번역기</p>
          <h1 className="mt-2 text-2xl font-bold text-white">제어 센터</h1>
          <p className="mt-2 text-sm text-slate-400">
            스팀 라이브러리를 점검하고 번역 작업을 실행하며 프로젝트 전반을 한눈에 살펴보세요.
          </p>
        </div>
        <nav className="flex-1 space-y-1 px-4 pb-6">
          {navigation.map((item) => (
            <NavLink
              key={item.to}
              to={item.to}
              end={item.to === '/'}
              className={({ isActive }) =>
                [
                  'block rounded-lg px-3 py-3 transition-all duration-150',
                  isActive
                    ? 'bg-brand-600/90 text-white shadow-lg shadow-brand-600/30'
                    : 'text-slate-300 hover:bg-slate-800/80 hover:text-white',
                ].join(' ')
              }
            >
              <div className="text-sm font-semibold">{item.label}</div>
              <div className="text-xs text-slate-400">{item.description}</div>
            </NavLink>
          ))}
        </nav>
      </aside>

      <main className="flex-1 overflow-y-auto">
        {showPolicyBanner && policyBanner && (
          <div
            className="border-b border-yellow-500/30 bg-yellow-500/5 px-6 py-5 text-sm text-yellow-100"
            role="alert"
          >
            <div className="mx-auto flex max-w-5xl flex-col gap-4">
              <div>
                <p className="text-xs font-semibold uppercase tracking-widest text-yellow-400">
                  {policyBanner.headline}
                </p>
                <p className="mt-1 text-sm text-yellow-100">{policyBanner.message}</p>
              </div>
              <label className="flex items-start gap-3 text-xs text-yellow-200">
                <input
                  type="checkbox"
                  checked={policyAcknowledged}
                  onChange={(event) => {
                    const nextValue = event.target.checked
                    setPolicyAcknowledgedState(nextValue)
                    persistPolicyAcknowledged(nextValue)
                  }}
                  className="mt-0.5 h-4 w-4 rounded border-yellow-400 bg-slate-950 text-yellow-500 focus:ring-yellow-400"
                />
                <span>{policyBanner.checkbox_label}</span>
              </label>
              <p className="text-xs text-yellow-300">{policyBanner.warning}</p>
            </div>
          </div>
        )}
        <header className="border-b border-slate-800/70 bg-slate-900/40 backdrop-blur">
          <div className="mx-auto flex max-w-5xl flex-col gap-4 px-6 py-6 md:flex-row md:items-center md:justify-between">
            <div>
              <h2 className="text-lg font-semibold text-white">Mod Translator 작업 공간</h2>
              <p className="text-sm text-slate-400">
                스팀과 연결하고 AI 번역기를 조율하며 품질을 모니터링하세요.
                {policyBanner && policyBanner.requires_acknowledgement
                  ? policyAcknowledged
                    ? ' 정책 동의가 기록되었습니다.'
                    : ' 정책 안내를 확인하고 동의해 주세요.'
                  : ''}
              </p>
              {error && <p className="mt-2 text-xs text-rose-300">{error}</p>}
            </div>
            <nav className="flex flex-wrap gap-2 md:hidden">
              {navigation.map((item) => (
                <NavLink
                  key={item.to}
                  to={item.to}
                  end={item.to === '/'}
                  className={({ isActive }) =>
                    [
                      'rounded-full px-4 py-2 text-sm transition',
                      isActive
                        ? 'bg-brand-600 text-white shadow shadow-brand-600/40'
                        : 'bg-slate-800 text-slate-300 hover:bg-slate-700 hover:text-white',
                    ].join(' ')
                  }
                >
                  {item.label}
                </NavLink>
              ))}
            </nav>
          </div>
        </header>

        <section className="mx-auto max-w-5xl px-6 py-10">
          <Routes>
            <Route path="/" element={<DashboardView />} />
            <Route path="/mods" element={<ModManagementView />} />
            <Route path="/progress" element={<ProgressView />} />
            <Route path="/progress/:modId" element={<ProgressView />} />
            <Route path="/settings" element={<SettingsView />} />
          </Routes>
        </section>
      </main>
    </div>
  )
}

function App() {
  return (
    <LibraryProvider>
      <JobProvider>
        <AppShell />
      </JobProvider>
    </LibraryProvider>
  )
}

export default App
