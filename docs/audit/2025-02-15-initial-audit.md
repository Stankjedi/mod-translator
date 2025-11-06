# Mod Translator Audit Baseline (2025-02-15)

This document captures the initial state of the repository ahead of the comprehensive audit described in the "Codex 개발지침" brief. It identifies configuration gaps, build/toolchain risks, and outlines the follow-up pull requests that will be required to satisfy the release-readiness checklist.

## Repository & Workspace Structure

- The Cargo workspace currently includes the Tauri shell and the Rust core crate only (`apps/desktop/src-tauri`, `core`).【F:Cargo.toml†L1-L6】
- The desktop frontend lives under `apps/desktop` and now shares the root pnpm workspace; redundant npm/yarn lockfiles were removed and are ignored going forward.【F:package.json†L1-L37】【F:.gitignore†L1-L24】
- Root-level `.editorconfig`와 `.gitattributes`가 추가되어 줄바꿈·공백 규칙이 통일되었습니다.【F:.editorconfig†L1-L14】【F:.gitattributes†L1-L17】

## Package Management & Tooling Baseline

- `package.json`과 `apps/desktop/package.json` 모두 `packageManager` 필드와 Node 20 엔진 범위를 선언하며, Tauri 빌드 훅도 pnpm 스크립트를 사용합니다.【F:package.json†L1-L37】【F:apps/desktop/package.json†L1-L35】【F:apps/desktop/src-tauri/tauri.conf.json†L1-L21】
- 루트 `pnpm-workspace.yaml`이 생성되어 모노레포 패키지가 정식 워크스페이스로 묶였습니다.【F:pnpm-workspace.yaml†L1-L3】
- Node 도구 체인은 Corepack+pnpm 조합으로 고정되고, Rust는 기존 버전을 유지합니다.【F:.npmrc†L1-L2】【F:package.json†L1-L37】

## Quality Gates & CI Status

- GitHub Actions 워크플로는 Node 20 + pnpm 설치, 프런트엔드 린트/타입검사/빌드, Rust fmt/clippy, Tauri 번들 생성과 아티팩트 업로드를 모두 수행하도록 재구성되었습니다.【F:.github/workflows/ci.yml†L1-L200】
- 워크플로 요약과 동일한 스크립트가 `package.json`에 수록되어 로컬에서도 `pnpm lint`, `pnpm build`, `pnpm tauri:build` 등을 그대로 사용할 수 있습니다.【F:package.json†L1-L37】

## Tauri Configuration Snapshot

- `tauri.conf.json`의 `beforeDevCommand`, `beforeBuildCommand`가 pnpm 워크스페이스 스크립트를 호출하도록 업데이트되었습니다.【F:apps/desktop/src-tauri/tauri.conf.json†L1-L21】
- The bundle targets are set to `"all"`; we must validate Windows/Linux builds explicitly during the audit.

## Immediate Action Items (PR Breakdown)

1. **Quality gates & CI hardening** – add pnpm workspace metadata, align `tauri.conf.json` commands with pnpm scripts, provision `.editorconfig`/`.gitattributes`, and extend CI to run pnpm install + lint/typecheck alongside cargo checks.
2. **Validator relaxed mode & auto-recovery** – introduce feature-flagged validation modes in the core crate, cover relaxed XML recovery semantics, and ship regression tests plus CLI fixtures.
3. **UI improvements** – implement the "전체 선택 해제" control and ensure accessibility metadata (ARIA labels, focus handling) aligns with the UX spec.
4. **Logging & documentation** – expand structured logging across the Rust core and frontend bridge, document user-facing recovery flows, and capture the audit report (including failure cases and reproduction notes).

Each pull request must produce accompanying test runs (Rust + TypeScript) and update the audit report with results, culminating in the final "종합 점검 보고서" once the checklist has been satisfied.
