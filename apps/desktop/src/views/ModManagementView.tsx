const mods = [
  {
    name: 'Immersive HUD Pack',
    game: 'Skyrim Special Edition',
    status: 'Ready',
    languages: ['English', 'Spanish'],
    policy: 'Creation Club redistribution blocked without explicit permission.',
    workshopPath: 'C:/Program Files (x86)/Steam/steamapps/workshop/content/489830/123456',
    warnings: ['Managed DLL detected – resource-first editing required.'],
  },
  {
    name: 'Community Translation Overhaul',
    game: 'Stardew Valley',
    status: 'Requires Scan',
    languages: ['English'],
    policy: 'ConcernedApe EULA allows personal backups only.',
    workshopPath: 'D:/SteamLibrary/steamapps/workshop/content/413150/998877',
    warnings: ['Unity AssetBundle detected – skipped during batch translation.'],
  },
  {
    name: 'Legacy Quest Log',
    game: 'Baldur’s Gate 3',
    status: 'Outdated',
    languages: ['English', 'Japanese', 'French'],
    policy: 'Larian redistribution policy requires author approval and EULA compliance.',
    workshopPath: 'D:/SteamLibrary/steamapps/workshop/content/1086940/112233',
    warnings: ['Placeholder drift detected in last QA run – rerun validator.'],
  },
]

function ModManagementView() {
  return (
    <div className="space-y-6">
      <header>
        <h2 className="text-xl font-semibold text-white">Installed Mods</h2>
        <p className="text-sm text-slate-400">
          Placeholder data showing how the management console can surface workshop entries and translation
          readiness.
        </p>
      </header>

      <div className="overflow-hidden rounded-2xl border border-slate-800/60 bg-slate-900/60">
        <table className="min-w-full divide-y divide-slate-800 text-sm text-slate-200">
          <thead className="bg-slate-900/80 text-xs uppercase tracking-wider text-slate-400">
            <tr>
              <th scope="col" className="px-4 py-3 text-left">
                Mod Name
              </th>
              <th scope="col" className="px-4 py-3 text-left">
                Game
              </th>
              <th scope="col" className="px-4 py-3 text-left">
                Languages
              </th>
              <th scope="col" className="px-4 py-3 text-left">
                Status
              </th>
              <th scope="col" className="px-4 py-3 text-left">
                Policy / Notes
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-slate-800/60">
            {mods.map((mod) => (
              <tr key={mod.name} className="hover:bg-slate-800/60">
                <td className="px-4 py-4 font-medium text-white">{mod.name}</td>
                <td className="px-4 py-4 text-slate-300">{mod.game}</td>
                <td className="px-4 py-4 text-slate-300">
                  <div className="flex flex-wrap gap-2">
                    {mod.languages.map((language) => (
                      <span key={language} className="rounded-full bg-slate-800 px-2 py-1 text-xs text-slate-300">
                        {language}
                      </span>
                    ))}
                  </div>
                </td>
                <td className="px-4 py-4">
                  <span className="rounded-full bg-brand-600/20 px-3 py-1 text-xs font-semibold text-brand-500">
                    {mod.status}
                  </span>
                </td>
                <td className="px-4 py-4 text-xs text-slate-300">
                  <p className="font-medium text-white">{mod.policy}</p>
                  <p className="mt-1 text-[11px] text-slate-500">Workshop path: {mod.workshopPath}</p>
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
      </div>

      <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6">
        <h3 className="text-lg font-semibold text-white">Next Steps</h3>
        <ul className="mt-3 space-y-2 text-sm text-slate-300">
          <li>Confirm game-specific policy profiles before queueing translation jobs.</li>
          <li>Use the library scan command to refresh workshop metadata when Steam installs new mods.</li>
          <li>Leverage placeholder validators when exporting builds that include managed DLL resources.</li>
        </ul>
      </section>
    </div>
  )
}

export default ModManagementView
