# Desktop UI (React + Tauri Host)

This package contains the React + Tailwind desktop frontend that is embedded into the Tauri shell. The UI mirrors the backend abstractions in the `core` crate and surfaces the legal, policy, and pipeline metadata required for safe workshop translation flows.

## Available screens

- **Dashboard** – overview cards for Steam discovery, token-bucket health, policy profiles, and pipeline stages.
- **Mod Management** – table of workshop mods enriched with policy warnings and detected asset notes.
- **Progress** – translation job cards highlighting queue snapshots, rate limiting, and quality-gate status.
- **Settings** – configuration for AI providers, Steam overrides, concurrency/token-bucket controls, and placeholder/resource rules.

Every screen is wired for Tailwind styling and can be connected to real Tauri commands via `@tauri-apps/api`.

## Development scripts

```bash
npm install           # install dependencies
npm run dev           # vite dev server (frontend only)
npm run tauri:dev     # launch the Tauri shell with the React app
npm run tauri:build   # package the desktop binary
```

## Legal banner

`src/App.tsx` renders a persistent Steam Workshop policy banner that must be acknowledged before exporting translations. The banner language matches the policy profile emitted by the backend (`PolicyBanner`).

## Styling

Tailwind CSS is enabled via `tailwind.config.ts` and `index.css`. Customize colors or typography in those files.

## Next steps

- Replace placeholder datasets in `src/views` with the results from `core` Tauri commands.
- Surface acknowledgement state from the policy banner before enabling export buttons.
- Extend settings to persist API keys and per-provider rate limits.
