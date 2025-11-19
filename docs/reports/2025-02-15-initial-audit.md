# Mod Translator 감사 기준선 (2025-02-15)

이 문서는 "Codex 개발지침" 브리핑에 설명된 포괄적인 감사를 앞두고 저장소의 초기 상태를 포착합니다. 구성상의 공백, 빌드/도구 체인 위험을 식별하고 릴리스 준비 체크리스트를 충족하기 위해 필요한 후속 풀 리퀘스트(PR)를 개략적으로 설명합니다.

## 저장소 및 워크스페이스 구조

- Cargo 워크스페이스는 현재 Tauri 셸과 Rust 코어 크레이트(`apps/desktop/src-tauri`, `core`)만 포함합니다.【F:Cargo.toml†L1-L6】
- 데스크톱 프론트엔드는 `apps/desktop` 아래에 있으며 이제 루트 pnpm 워크스페이스를 공유합니다. 중복된 npm/yarn 잠금 파일은 제거되었으며 앞으로 무시됩니다.【F:package.json†L1-L37】【F:.gitignore†L1-L24】
- 루트 레벨 `.editorconfig`와 `.gitattributes`가 추가되어 줄바꿈·공백 규칙이 통일되었습니다.【F:.editorconfig†L1-L14】【F:.gitattributes†L1-L17】

## 패키지 관리 및 도구 기준선

- `package.json`과 `apps/desktop/package.json` 모두 `packageManager` 필드와 Node 20 엔진 범위를 선언하며, Tauri 빌드 훅도 pnpm 스크립트를 사용합니다.【F:package.json†L1-L37】【F:apps/desktop/package.json†L1-L35】【F:apps/desktop/src-tauri/tauri.conf.json†L1-L21】
- 루트 `pnpm-workspace.yaml`이 생성되어 모노레포 패키지가 정식 워크스페이스로 묶였습니다.【F:pnpm-workspace.yaml†L1-L3】
- Node 도구 체인은 Corepack+pnpm 조합으로 고정되고, Rust는 기존 버전을 유지합니다.【F:.npmrc†L1-L2】【F:package.json†L1-L37】

## 품질 게이트 및 CI 상태

- GitHub Actions 워크플로는 Node 20 + pnpm 설치, 프런트엔드 린트/타입검사/빌드, Rust fmt/clippy, Tauri 번들 생성과 아티팩트 업로드를 모두 수행하도록 재구성되었습니다.【F:.github/workflows/ci.yml†L1-L200】
- 워크플로 요약과 동일한 스크립트가 `package.json`에 수록되어 로컬에서도 `pnpm lint`, `pnpm build`, `pnpm tauri:build` 등을 그대로 사용할 수 있습니다.【F:package.json†L1-L37】

## Tauri 구성 스냅샷

- `tauri.conf.json`의 `beforeDevCommand`, `beforeBuildCommand`가 pnpm 워크스페이스 스크립트를 호출하도록 업데이트되었습니다.【F:apps/desktop/src-tauri/tauri.conf.json†L1-L21】
- 번들 타겟은 `"all"`로 설정되어 있습니다. 감사 중에 Windows/Linux 빌드를 명시적으로 검증해야 합니다.

## 즉각적인 조치 항목 (PR 세분화)

1. **품질 게이트 및 CI 강화** – pnpm 워크스페이스 메타데이터 추가, `tauri.conf.json` 명령을 pnpm 스크립트와 정렬, `.editorconfig`/`.gitattributes` 프로비저닝, 그리고 CI를 확장하여 cargo 검사와 함께 pnpm install + lint/typecheck를 실행하도록 합니다.
2. **검증기 완화 모드 및 자동 복구** – 코어 크레이트에 기능 플래그가 지정된 검증 모드를 도입하고, 완화된 XML 복구 의미론을 다루며, 회귀 테스트 및 CLI 픽스처를 제공합니다.
3. **UI 개선** – "전체 선택 해제" 컨트롤을 구현하고 접근성 메타데이터(ARIA 레이블, 포커스 처리)가 UX 사양과 일치하는지 확인합니다.
4. **로깅 및 문서화** – Rust 코어 및 프론트엔드 브리지 전반에 걸쳐 구조화된 로깅을 확장하고, 사용자 대면 복구 흐름을 문서화하며, 감사 보고서(실패 사례 및 재현 노트 포함)를 캡처합니다.

각 풀 리퀘스트는 동반되는 테스트 실행(Rust + TypeScript)을 생성하고 감사 보고서를 결과로 업데이트해야 하며, 체크리스트가 충족되면 최종 "종합 점검 보고서"로 정점을 찍습니다.
