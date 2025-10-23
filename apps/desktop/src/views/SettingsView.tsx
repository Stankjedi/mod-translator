function SettingsView() {
  return (
    <div className="space-y-8">
      <header>
        <h2 className="text-xl font-semibold text-white">Workspace Settings</h2>
        <p className="text-sm text-slate-400">
          Configure translator preferences, Steam integration, and logging. These controls are placeholders that map
          to the backend surface area defined in the Rust workspace.
        </p>
      </header>

      <form className="space-y-6">
        <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30">
          <h3 className="text-lg font-semibold text-white">AI Providers</h3>
          <p className="mt-1 text-sm text-slate-400">
            Toggle which translator implementations should be available when orchestrating jobs.
          </p>
          <div className="mt-4 grid gap-4 sm:grid-cols-2">
            <label className="flex items-center gap-3 rounded-xl border border-slate-800/60 bg-slate-950/60 p-4">
              <input type="checkbox" defaultChecked className="h-4 w-4 rounded border-slate-700 bg-slate-900" />
              <span>
                <span className="block text-sm font-medium text-white">Gemini Advanced</span>
                <span className="text-xs text-slate-400">Uses Google-hosted context aware models.</span>
              </span>
            </label>
            <label className="flex items-center gap-3 rounded-xl border border-slate-800/60 bg-slate-950/60 p-4">
              <input type="checkbox" defaultChecked className="h-4 w-4 rounded border-slate-700 bg-slate-900" />
              <span>
                <span className="block text-sm font-medium text-white">GPT-4.1 Turbo</span>
                <span className="text-xs text-slate-400">High quality output with extended context windows.</span>
              </span>
            </label>
            <label className="flex items-center gap-3 rounded-xl border border-slate-800/60 bg-slate-950/60 p-4">
              <input type="checkbox" className="h-4 w-4 rounded border-slate-700 bg-slate-900" />
              <span>
                <span className="block text-sm font-medium text-white">Claude 3.5 Sonnet</span>
                <span className="text-xs text-slate-400">Anthropic adapter tuned for dialogue-heavy strings.</span>
              </span>
            </label>
            <label className="flex items-center gap-3 rounded-xl border border-slate-800/60 bg-slate-950/60 p-4">
              <input type="checkbox" className="h-4 w-4 rounded border-slate-700 bg-slate-900" />
              <span>
                <span className="block text-sm font-medium text-white">xAI Grok 2</span>
                <span className="text-xs text-slate-400">Experimental provider for rapid iteration.</span>
              </span>
            </label>
          </div>
        </section>

        <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30">
          <h3 className="text-lg font-semibold text-white">Steam Integration</h3>
          <div className="mt-4 space-y-4">
            <label className="block text-sm font-medium text-slate-300">Steam Install Path</label>
            <input
              type="text"
              defaultValue="C:/Program Files (x86)/Steam"
              className="w-full rounded-xl border border-slate-800 bg-slate-950 px-4 py-3 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
            />
            <button
              type="button"
              className="inline-flex items-center justify-center rounded-xl bg-brand-600 px-4 py-2 text-sm font-semibold text-white shadow shadow-brand-600/40 transition hover:bg-brand-500"
            >
              Detect Automatically
            </button>
          </div>
        </section>

        <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30">
          <h3 className="text-lg font-semibold text-white">Concurrency &amp; Rate Limits</h3>
          <p className="mt-1 text-sm text-slate-400">
            Configure the worker queue and token bucket to respect provider quotas.
          </p>
          <div className="mt-4 grid gap-4 sm:grid-cols-3">
            <label className="flex flex-col gap-2 text-sm text-slate-300">
              <span>Concurrent Workers</span>
              <input
                type="number"
                defaultValue={3}
                min={1}
                className="rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
              />
            </label>
            <label className="flex flex-col gap-2 text-sm text-slate-300">
              <span>Token Bucket Capacity</span>
              <input
                type="number"
                defaultValue={5}
                min={1}
                className="rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
              />
            </label>
            <label className="flex flex-col gap-2 text-sm text-slate-300">
              <span>Refill Interval (ms)</span>
              <input
                type="number"
                defaultValue={750}
                min={100}
                step={50}
                className="rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
              />
            </label>
          </div>
        </section>

        <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30">
          <h3 className="text-lg font-semibold text-white">Translation Rules &amp; Logging</h3>
          <div className="mt-4 space-y-4 text-sm text-slate-300">
            <label className="flex items-center justify-between gap-3">
              <span>Verbose Backend Logging</span>
              <input type="checkbox" className="h-4 w-4 rounded border-slate-700 bg-slate-900" />
            </label>
            <label className="flex items-center justify-between gap-3">
              <span>Enforce placeholder parity validator</span>
              <input type="checkbox" defaultChecked className="h-4 w-4 rounded border-slate-700 bg-slate-900" />
            </label>
            <label className="flex items-center justify-between gap-3">
              <span>Resource-first DLL handling (Mono.Cecil)</span>
              <input type="checkbox" defaultChecked className="h-4 w-4 rounded border-slate-700 bg-slate-900" />
            </label>
            <label className="flex items-center justify-between gap-3">
              <span>Trigger quality sampling (5%)</span>
              <input type="checkbox" defaultChecked className="h-4 w-4 rounded border-slate-700 bg-slate-900" />
            </label>
          </div>
        </section>
      </form>
    </div>
  )
}

export default SettingsView
