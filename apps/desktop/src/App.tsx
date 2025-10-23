import { useState } from 'react'
import { NavLink, Route, Routes } from 'react-router-dom'
import DashboardView from './views/DashboardView'
import ModManagementView from './views/ModManagementView'
import ProgressView from './views/ProgressView'
import SettingsView from './views/SettingsView'

const navigation = [
  { to: '/', label: 'Dashboard', description: 'Overview of recent activity' },
  { to: '/mods', label: 'Mod Management', description: 'Manage installed workshop content' },
  { to: '/progress', label: 'Progress', description: 'Track translation pipelines' },
  { to: '/settings', label: 'Settings', description: 'Configure translator preferences' },
]

function App() {
  const [policyAcknowledged, setPolicyAcknowledged] = useState(false)

  return (
    <div className="flex min-h-screen bg-slate-950 text-slate-100">
      <aside className="hidden w-72 flex-col border-r border-slate-800/60 bg-slate-900/60 backdrop-blur md:flex">
        <div className="px-6 py-8">
          <p className="text-sm font-semibold uppercase tracking-widest text-slate-400">Mod Translator</p>
          <h1 className="mt-2 text-2xl font-bold text-white">Control Center</h1>
          <p className="mt-2 text-sm text-slate-400">
            Launch translation jobs, inspect your Steam library, and keep projects on track.
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
        <div
          className="border-b border-yellow-500/30 bg-yellow-500/5 px-6 py-5 text-sm text-yellow-100"
          role="alert"
        >
          <div className="mx-auto flex max-w-5xl flex-col gap-4">
            <div>
              <p className="text-xs font-semibold uppercase tracking-widest text-yellow-400">
                Steam Workshop Policy Notice
              </p>
              <p className="mt-1 text-sm text-yellow-100">
                Workshop assets are provided for personal use only. Redistribution of localized builds requires explicit
                permission from the original author. Steam/Community Guidelines warn that unauthorized distribution can
                lead to account restrictions.
              </p>
            </div>
            <label className="flex items-start gap-3 text-xs text-yellow-200">
              <input
                type="checkbox"
                checked={policyAcknowledged}
                onChange={(event) => setPolicyAcknowledged(event.target.checked)}
                className="mt-0.5 h-4 w-4 rounded border-yellow-400 bg-slate-950 text-yellow-500 focus:ring-yellow-400"
              />
              <span>
                I understand that redistribution requires permission and agree to comply with all game-specific EULAs.
              </span>
            </label>
          </div>
        </div>
        <header className="border-b border-slate-800/70 bg-slate-900/40 backdrop-blur">
          <div className="mx-auto flex max-w-5xl flex-col gap-4 px-6 py-6 md:flex-row md:items-center md:justify-between">
            <div>
              <h2 className="text-lg font-semibold text-white">Mod Translator Workspace</h2>
              <p className="text-sm text-slate-400">
                Connect to Steam, orchestrate AI translators, and monitor localization health from one place.
                {policyAcknowledged ? ' Policy acknowledgement recorded.' : ' Confirm the policy notice before exporting translations.'}
              </p>
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
            <Route path="/settings" element={<SettingsView />} />
          </Routes>
        </section>
      </main>
    </div>
  )
}

export default App
