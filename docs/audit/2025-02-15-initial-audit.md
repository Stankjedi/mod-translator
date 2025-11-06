# Mod Translator Audit Baseline (2025-02-15)

This document captures the initial state of the repository ahead of the comprehensive audit described in the "Codex 개발지침" brief. It identifies configuration gaps, build/toolchain risks, and outlines the follow-up pull requests that will be required to satisfy the release-readiness checklist.

## Repository & Workspace Structure

- The Cargo workspace currently includes the Tauri shell and the Rust core crate only (`apps/desktop/src-tauri`, `core`).【F:Cargo.toml†L1-L6】
- The desktop frontend lives under `apps/desktop` and ships both `pnpm-lock.yaml` and `package-lock.json`, signalling active usage of multiple package managers.【F:apps/desktop/pnpm-lock.yaml†L1-L8】【F:apps/desktop/package-lock.json†L1-L14】
- No root-level `.editorconfig` or `.gitattributes` file is present, so contributors rely on personal editor defaults, risking inconsistent line endings and whitespace.【F:.gitignore†L1-L29】

## Package Management & Tooling Baseline

- `package.json` does not declare a `packageManager` field, and the Tauri build hooks rely on plain `npm` (`beforeDevCommand`, `beforeBuildCommand`), which contradicts the single-tool `pnpm` requirement.【F:apps/desktop/package.json†L1-L35】【F:apps/desktop/src-tauri/tauri.conf.json†L7-L29】
- There is no repository-level `pnpm-workspace.yaml`, so the monorepo layout is not formally wired up for pnpm workspaces.
- Node/Rust toolchain pins are absent outside of `rust-version = "1.77.2"` in the Tauri crate, leaving contributors to resolve tool versions manually.【F:apps/desktop/src-tauri/Cargo.toml†L1-L27】

## Quality Gates & CI Status

- The only GitHub Actions workflow (`CI`) installs a Rust toolchain and runs `cargo fmt` plus `cargo clippy`; there are no frontend lint/type checks, no `pnpm` install, and no build artefacts captured.【F:.github/workflows/ci.yml†L1-L32】
- There are no documented commands for `cargo test`, `pnpm lint`, `pnpm build`, or tauri bundle smoke tests in the workflow. The audit will need dedicated jobs to cover formatting, linting, type-checking, bundling, and artefact upload per the checklist.

## Tauri Configuration Snapshot

- `tauri.conf.json` points to `icons/icon.ico`, which exists, but the command hooks do not align with the desired pnpm-based flow.【F:apps/desktop/src-tauri/tauri.conf.json†L7-L29】
- The bundle targets are set to `"all"`; we must validate Windows/Linux builds explicitly during the audit.

## Immediate Action Items (PR Breakdown)

1. **Quality gates & CI hardening** – add pnpm workspace metadata, align `tauri.conf.json` commands with pnpm scripts, provision `.editorconfig`/`.gitattributes`, and extend CI to run pnpm install + lint/typecheck alongside cargo checks.
2. **Validator relaxed mode & auto-recovery** – introduce feature-flagged validation modes in the core crate, cover relaxed XML recovery semantics, and ship regression tests plus CLI fixtures.
3. **UI improvements** – implement the "전체 선택 해제" control and ensure accessibility metadata (ARIA labels, focus handling) aligns with the UX spec.
4. **Logging & documentation** – expand structured logging across the Rust core and frontend bridge, document user-facing recovery flows, and capture the audit report (including failure cases and reproduction notes).

Each pull request must produce accompanying test runs (Rust + TypeScript) and update the audit report with results, culminating in the final "종합 점검 보고서" once the checklist has been satisfied.
