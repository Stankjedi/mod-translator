const metrics = [
  {
    title: 'Detected Libraries',
    value: '3',
    hint: 'Registry + VDF parse completed 2 minutes ago',
  },
  {
    title: 'Active Tokens',
    value: '4 / 5',
    hint: 'Token bucket refills every 750ms',
  },
  {
    title: 'Quality Samples',
    value: '5%',
    hint: 'Automatic re-requests triggered for failed checks',
  },
]

const highlights = [
  {
    title: 'Steam Sync',
    description:
      'Library scan successfully parsed libraryfolders.vdf and discovered two workshop content roots.',
  },
  {
    title: 'AI Translator Health',
    description:
      'Gemini, GPT, Claude, and Grok adapters are online with placeholder guards enabled for every batch.',
  },
]

const policyProfiles = [
  {
    game: 'Skyrim Special Edition',
    notes:
      'Creation Club terms apply. Redistribution blocked without the original author and Bethesda approval.',
  },
  {
    game: 'Baldur’s Gate 3',
    notes: 'Larian EULA requires personal-use only exports. Community reuploads must be negotiated.',
  },
]

const pipelineStages = [
  'Unpack workshop archive',
  'Detect file formats and classify text assets',
  'Parse JSON/INI/XML/RESX resources',
  'Translate batches with placeholder locking',
  'Run placeholder + markup validators',
  'Repackage resources or Harmony patches',
]

const quickActions = [
  { label: 'Run Library Scan', description: 'Parse registry + libraryfolders.vdf for new libraries.' },
  { label: 'Start Translation Job', description: 'Queue a batch with token bucket + retry guard.' },
  { label: 'Review Policy Profiles', description: 'Confirm redistribution requirements before export.' },
]

function DashboardView() {
  return (
    <div className="space-y-10">
      <section>
        <h2 className="text-xl font-semibold text-white">Today&apos;s Overview</h2>
        <p className="text-sm text-slate-400">
          A snapshot of the desktop orchestrator&apos;s status including policy, pipeline, and rate-limit health.
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
          <h3 className="text-lg font-semibold text-white">Operational Highlights</h3>
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
          <h3 className="text-lg font-semibold text-white">Quick Actions</h3>
          <ul className="mt-4 space-y-3 text-sm text-slate-300">
            {quickActions.map((action) => (
              <li
                key={action.label}
                className="flex items-start justify-between rounded-lg border border-slate-800/60 bg-slate-900/60 p-4"
              >
                <div>
                  <div className="font-medium text-white">{action.label}</div>
                  <p className="mt-1 text-xs text-slate-400">{action.description}</p>
                </div>
                <span className="rounded-full bg-brand-600/20 px-3 py-1 text-xs font-semibold text-brand-500">
                  Launch
                </span>
              </li>
            ))}
          </ul>
        </div>
      </section>

      <section className="grid gap-6 lg:grid-cols-2">
        <div className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6">
          <h3 className="text-lg font-semibold text-white">Game-specific Policy Profiles</h3>
          <ul className="mt-4 space-y-3 text-sm text-slate-300">
            {policyProfiles.map((profile) => (
              <li key={profile.game} className="rounded-xl border border-slate-800/60 bg-slate-900/60 p-4">
                <div className="text-sm font-semibold text-white">{profile.game}</div>
                <p className="mt-1 text-xs text-slate-400">{profile.notes}</p>
              </li>
            ))}
          </ul>
        </div>
        <div className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6">
          <h3 className="text-lg font-semibold text-white">Pipeline Snapshot</h3>
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
