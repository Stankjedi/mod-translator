const jobs = [
  {
    id: 'JOB-4021',
    mod: 'Immersive HUD Pack',
    translator: 'Gemini Advanced',
    status: 'Completed',
    progress: 100,
    message: 'All files translated and validated. Placeholder parity confirmed.',
    queue: { queued: 0, running: 1, capacity: 3 },
    rateLimiter: { tokens: 4, capacity: 5, interval: '750ms' },
    quality: { placeholderGuard: true, sampleRate: '5%' },
  },
  {
    id: 'JOB-4022',
    mod: 'Legacy Quest Log',
    translator: 'GPT-4.1 Turbo',
    status: 'Processing',
    progress: 65,
    message: 'Batch 4/6 in-flight with DLL resource reinsertion pending.',
    queue: { queued: 1, running: 3, capacity: 3 },
    rateLimiter: { tokens: 2, capacity: 5, interval: '750ms' },
    quality: { placeholderGuard: true, sampleRate: '5%' },
  },
  {
    id: 'JOB-4023',
    mod: 'Community Translation Overhaul',
    translator: 'Claude 3.5 Sonnet',
    status: 'Queued',
    progress: 0,
    message: 'Waiting for token bucket refill and glossary confirmation.',
    queue: { queued: 2, running: 3, capacity: 3 },
    rateLimiter: { tokens: 0, capacity: 5, interval: '750ms' },
    quality: { placeholderGuard: true, sampleRate: '5%' },
  },
]

function ProgressView() {
  return (
    <div className="space-y-6">
      <header>
        <h2 className="text-xl font-semibold text-white">Translation Progress</h2>
        <p className="text-sm text-slate-400">
          Placeholder job cards demonstrate queue snapshots, rate limiting, and quality guard rails surfaced from the
          backend orchestrator.
        </p>
      </header>

      <div className="space-y-4">
        {jobs.map((job) => (
          <article
            key={job.id}
            className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30"
          >
            <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
              <div>
                <h3 className="text-lg font-semibold text-white">{job.mod}</h3>
                <p className="text-xs uppercase tracking-wider text-slate-500">{job.id}</p>
                <p className="mt-1 text-sm text-slate-400">Translator: {job.translator}</p>
              </div>
              <div className="text-right">
                <span className="rounded-full bg-brand-600/20 px-3 py-1 text-xs font-semibold text-brand-500">
                  {job.status}
                </span>
              </div>
            </div>
            <div className="mt-4">
              <div className="h-2 w-full overflow-hidden rounded-full bg-slate-800">
                <div
                  className="h-full rounded-full bg-brand-600"
                  style={{ width: `${job.progress}%` }}
                />
              </div>
              <p className="mt-2 text-xs text-slate-400">{job.message}</p>
              <div className="mt-3 grid gap-2 text-[11px] text-slate-400 sm:grid-cols-3">
                <div className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-3">
                  <p className="font-semibold text-slate-200">Queue</p>
                  <p>
                    {job.queue.running}/{job.queue.capacity} workers • {job.queue.queued} queued
                  </p>
                </div>
                <div className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-3">
                  <p className="font-semibold text-slate-200">Rate Limiter</p>
                  <p>
                    {job.rateLimiter.tokens}/{job.rateLimiter.capacity} tokens • refill {job.rateLimiter.interval}
                  </p>
                </div>
                <div className="rounded-lg border border-slate-800/60 bg-slate-900/60 p-3">
                  <p className="font-semibold text-slate-200">Quality Gates</p>
                  <p>Placeholders locked: {job.quality.placeholderGuard ? 'Yes' : 'No'}</p>
                  <p>Sampling: {job.quality.sampleRate}</p>
                </div>
              </div>
            </div>
          </article>
        ))}
      </div>
    </div>
  )
}

export default ProgressView
