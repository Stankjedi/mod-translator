# ModSync Translator Workspace

The workspace bundles a policy-aware Tauri desktop shell with a React + Tailwind UI and a Rust backend that models Steam Workshop discovery, translation pipelines, and AI adapter abstractions.

## Layout

```
.
├── apps/desktop/          # React + TypeScript frontend with Tailwind CSS
│   ├── src-tauri/         # Tauri host application
│   └── src/views/         # Dashboard, mod management, progress, and settings screens
├── core/                  # Shared Rust library (Steam discovery, policy, pipeline, AI adapters)
└── Cargo.toml             # Workspace definition
```

## Prerequisites

- Node.js 18+ (Vite/Tailwind tooling)
- Rust 1.77+ with `tauri-cli`
- npm (or pnpm/yarn)

## First-time setup

```bash
cd apps/desktop
npm install
```

## Run the desktop workspace in development

```bash
cd apps/desktop
npm run tauri:dev
```

This launches the Vite dev server, the Tauri shell, and wires the React UI to the Rust commands from `core`.

## Build for release

```bash
cd apps/desktop
npm run build       # bundle the React frontend
npm run tauri:build # package the desktop app
```

## Backend architecture

- `core/src/steam.rs` discovers Steam via environment overrides, Windows registry lookup, and `libraryfolders.vdf` parsing. The scanner surfaces workshop/content roots plus helper methods for app manifests.
- `core/src/library.rs` enumerates workshop directories, synthesizes policy-rich `ModSummary` records, flags DLL/binary assets, and attaches conservative `PolicyProfile` guidance for each game.
- `core/src/ai/mod.rs` defines a `Translator` trait with a provider-agnostic `translate_batch` API, placeholder guards, and stub adapters for Gemini, GPT, Claude, and Grok. `TranslateOptions` captures domain/style metadata for downstream providers.
- `core/src/jobs.rs` models a work queue, token-bucket rate limiter, quality gate snapshot, and pipeline plan summary for each translation request.
- `core/src/pipeline.rs` and `core/src/policy.rs` describe the translation stages, validator specs, and legal banner content shared with the UI.

### Exposed Tauri commands

- `discover_steam_path` → `SteamPathResponse`
- `scan_library` → `LibraryScanResponse` (includes `PolicyBanner` data for the UI banner)
- `start_translation_job` → `TranslationJobStatus` with queue, rate-limit, and pipeline snapshots

## Frontend architecture

- `src/App.tsx` renders the navigation shell and a mandatory Steam Workshop legal banner (headline, warning, and acknowledgement checkbox).
- `views/DashboardView.tsx` surfaces policy profiles, pipeline stages, and concurrency metrics.
- `views/ModManagementView.tsx` presents workshop mod placeholders enriched with policy warnings and detected asset notes.
- `views/ProgressView.tsx` shows queue, rate limit, and quality gating data alongside job progress.
- `views/SettingsView.tsx` lets users configure translators, Steam overrides, concurrency limits, and translation safeguards (placeholder parity, resource-first DLL handling, etc.).

Tailwind CSS powers the layout; React Router (`HashRouter`) keeps routes stable in the Tauri environment.

## Extending the workspace

- Expand Steam/workshop logic in `core::steam` and `core::library` to read real manifests or integrate additional policy profiles.
- Implement provider integrations by replacing stub translators in `core::ai` while keeping `TranslateOptions` stable.
- Drive the UI from real Tauri command results via `@tauri-apps/api` invoke calls.
- Keep legal and policy messaging intact—any feature that exports content should require the acknowledgement surfaced in the banner.

