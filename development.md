# ModSync Translator 워크스페이스

이 워크스페이스는 정책 검증을 지원하는 Tauri 데스크톱 셸과 React + Tailwind UI, Steam 워크숍 탐색과 번역 파이프라인, AI 어댑터 추상화를 모델링하는 Rust 백엔드를 함께 제공합니다.

## 레이아웃

```
.
├── apps/desktop/          # React + TypeScript 프런트엔드(Tailwind CSS 사용)
│   ├── src-tauri/         # Tauri 호스트 애플리케이션
│   └── src/views/         # 대시보드·모드 관리·진행 상황·설정 화면
├── core/                  # 공유 Rust 라이브러리(워크숍 탐색, 정책, 파이프라인, AI 어댑터)
└── Cargo.toml             # 워크스페이스 정의
```

## 사전 준비

- Node.js 18+ (Vite/Tailwind 도구 체인)
- Rust 1.77+ 및 `tauri-cli`
- npm (또는 pnpm/yarn)

### Windows 전용 설정

Windows 타깃에서 Rust 크레이트를 빌드하려면 Microsoft Visual C++ 도구 체인이 필요합니다. `linker link.exe not found`와 같은 오류가 표시되면 **Visual Studio 2017 이상용 Build Tools**(또는 전체 Visual Studio) 설치 후 **Desktop development with C++** 워크로드를 포함하세요. 설치가 끝나면 터미널을 재시작하고 `link.exe`가 `PATH`에 있는지 확인한 뒤 `rustup default stable-x86_64-pc-windows-msvc`(또는 `rustup target add x86_64-pc-windows-msvc`)를 실행해 Rust가 MSVC 도구 체인을 사용하도록 설정합니다.

## 최초 설정

```bash
cd apps/desktop
npm install
```

## 개발 모드 실행

```bash
cd apps/desktop
npm run tauri:dev
```

이 명령은 Vite 개발 서버와 Tauri 셸을 실행하고 React UI를 `core`의 Rust 명령과 연결합니다. 프런트엔드만 빠르게 조정하려면 `npm run dev`
를 사용해 브라우저에서 UI를 확인할 수 있습니다.

## 릴리스 빌드

```bash
cd apps/desktop
npm run build       # React 프런트엔드 번들링
npm run tauri:build # 데스크톱 앱 패키징
```

Windows용 MSI 패키지는 `apps/desktop/src-tauri/icons/icon.ico`에 있는 아이콘을 사용합니다. 다른 형식(PNG/SVG/ICNS)만 배치하면 MSIX/ MSI
빌드가 실패하므로 `.ico` 파일을 유지하세요.

## 번역 플로우 개요

1. **Scan** – `detect_steam_path`와 `scan_steam_library`가 Steam 라이브러리 후보를 찾고 워크샵 콘텐츠를 열거합니다. 모든 경로 변환은
   UTF-8 실패 시 UI로 오류를 반환합니다.
2. **Plan** – `PipelinePlan::default_for`가 단계, 검증자, 생략 규칙을 정의해 UI에 전달합니다.
3. **Execute** – `start_translation_job`이 작업 큐, 속도 제한, 품질 게이트 스냅샷을 생성하고 미리보기 번역을 제공합니다.
4. **Output & Backup** – 변환된 파일을 내보내기 전 원본을 백업(아카이브 복제)하고, 변환 단계는 DLL/바이너리 추출 정책을 준수합니다
   (상세 전략은 `docs/production-hardening-plan.md` 참고).

Rust 코어는 작업 로그를 `%APPDATA%/mod-translator/logs/jobs.log`(Windows 기준) 아래에 JSONL로 보존해 비정상 종료 후에도 상태를 복원할
수 있습니다.

## 환경 및 키 관리

- API 키는 UI 설정 화면에서 입력해 로컬 스토리지에만 저장합니다. 저장된 값은 암호화되지 않으므로 필요 시 OS 수준 암호화 저장소로
  이전하세요.
- 공급자 인증 정보는 `TranslationJobRequest.provider_auth`에 직렬화되어 전달됩니다. 실제 런타임에서는 `tauri::Plugin` 또는 OS 자격
  증명 관리자를 연결해 보안을 강화하세요.
- 저장소에는 비밀 키를 커밋하지 않도록 `.gitignore`에 `*.env.local`, `apps/desktop/.env`, `apps/desktop/src-tauri/.env` 등의 항목이 포
  함되어야 합니다.

## 백엔드 구조

- `core/src/steam.rs`: 환경 변수, Windows 레지스트리 조회, `libraryfolders.vdf` 파싱을 통해 Steam을 탐색하고 워크숍/콘텐츠 루트와 앱 매니페스트 헬퍼를 노출합니다.
- `core/src/library.rs`: 워크숍 디렉터리를 열거하고 정책 정보가 포함된 `ModSummary`를 생성하며 DLL/바이너리 자산을 표시하고 각 게임에 보수적인 `PolicyProfile` 가이던스를 부여합니다.
- `core/src/ai/mod.rs`: 공급자에 구애받지 않는 `translate_batch` API를 가진 `Translator` 트레이트와 Gemini, GPT, Claude, Grok용 스텁 어댑터, 도메인/스타일 메타데이터를 담는 `TranslateOptions`를 정의합니다.
- `core/src/jobs.rs`: 작업 큐, 토큰 버킷 속도 제한기, 품질 게이트 스냅샷, 각 번역 요청의 파이프라인 계획 요약을 모델링합니다. 작업
  상태는 JSONL 로그로 기록되어 재시작 시 복원 근거로 사용할 수 있습니다.
- `core/src/pipeline.rs`, `core/src/policy.rs`: 번역 단계, 검증자 사양, UI 배너와 공유하는 법적 안내 문구를 기술합니다.

### 공개된 Tauri 명령

- `detect_steam_path` → `SteamPathResponse`
- `scan_steam_library` → `LibraryScanResponse` (`PolicyBanner` 데이터를 포함해 UI 배너에 전달)
- `start_translation_job` → 큐·속도 제한·파이프라인 스냅샷을 담은 `TranslationJobStatus`

## 프런트엔드 구조

- `src/App.tsx`: 탐색 셸과 필수 Steam 워크숍 법적 배너(헤드라인, 경고, 확인 체크박스)를 렌더링합니다.
- `views/DashboardView.tsx`: 정책 프로필, 파이프라인 단계, 동시성 지표를 제공합니다.
- `views/ModManagementView.tsx`: 워크숍 모드 자리표시자를 표시하고 정책 경고와 탐지된 자산 정보를 덧붙입니다.
- `views/ProgressView.tsx`: 큐, 속도 제한, 품질 게이트 데이터를 작업 진행 상황과 함께 보여줍니다.
- `views/SettingsView.tsx`: 번역기, Steam 경로 재정의, 동시성 제한, 번역 보호장치(자리표시자 유지, DLL 우선 처리 등)를 구성합니다.

레이아웃은 Tailwind CSS가 담당하며, React Router(`HashRouter`)는 Tauri 환경에서도 안정적인 라우팅을 제공합니다.

## 워크스페이스 확장 방법

- 실제 매니페스트를 읽거나 추가 정책 프로필을 통합하려면 `core::steam`과 `core::library`의 Steam/워크숍 로직을 확장합니다.
- `core::ai`의 스텁 번역기를 실제 공급자 통합으로 교체하되 `TranslateOptions`는 유지합니다.
- `@tauri-apps/api`의 invoke 호출을 통해 UI를 실제 Tauri 명령 결과에 연결합니다. TypeScript 인터페이스는 `apps/desktop/src/types/core.ts`
  에 정의되어 Rust 직렬화 구조체와 1:1로 대응합니다.
- 법적·정책 안내 문구를 유지하세요. 콘텐츠를 내보내는 기능은 배너에 제공되는 확인 절차를 반드시 요구해야 합니다.
