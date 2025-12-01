# 🤖 AI Agent Improvement Prompts

> ## 🚨 MANDATORY: FILE MODIFICATION REQUIRED
> 
> **⛔ DO NOT just respond with text or code blocks.**
> **⛔ DO NOT say "you can modify like this" without actually modifying.**
> **⛔ DO NOT explain what needs to be done without doing it.**
> 
> **✅ YOU MUST use file editing tools (`replace_string_in_file`, `create_file`, `multi_replace_string_in_file`) to directly modify files.**
> **✅ After each prompt, confirm "I have modified the file(s)" with the actual changes made.**
> 
> ---
> 
> **Process ALL prompts below SEQUENTIALLY. Do NOT stop after completing one prompt.**

---

## 📋 Execution Checklist

| # | Prompt ID | Title | Priority | Status |
|:---:|:---|:---|:---:|:---:|
| 1 | PROMPT-001 | -1] API 키 저장소 보안 강화 (Tauri Stronghold) | P1 | ⬜ Pending |
| 2 | PROMPT-002 | -1] E2E 테스트 스위트 추가 | P2 | ⬜ Pending |
| 3 | PROMPT-003 | -2] UI 에러 피드백 상세화 | P2 | ⬜ Pending |
| 4 | PROMPT-004 | -3] 번역 제외(ignore) 기능 | P2 | ⬜ Pending |
| 5 | PROMPT-005 | -1] 다국어 UI 지원 (i18n) | P3 | ⬜ Pending |

**Total: 5 prompts** | **Completed: 0** | **Remaining: 5**

---

## 🔴 Priority 1 (Critical) - Execute First

### [PROMPT-001] -1] API 키 저장소 보안 강화 (Tauri Stronghold)

**⏱️ Execute this prompt now, then proceed to PROMPT-002**

> **🚨 REQUIRED: Use `replace_string_in_file` or `create_file` to make changes. Do NOT just show code.**

**Task**: -1] API 키 저장소 Security 강화 (Tauri Stronghold)

**Details:**

| 항목 | 내용 |
|:---|:---|
| **ID** | `SEC-001` |
| **Category** | 🔒 보안 |
| **Complexity** | Medium |
| **Target Files** | `apps/desktop/src/storage/apiKeyStorage.ts`, `apps/desktop/src-tauri/Cargo.toml`, `apps/desktop/src-tauri/src/lib.rs` |

**Current State:** API 키가 브라우저의 `localStorage`에 일반 텍스트로 저장됩니다. CSP 정책으로 외부 접근은 차단되지만, 로컬 파일 시스템에서 데이터 추출이 가능합니다.

**Improvement:** `tauri-plugin-stronghold`를 도입하여 API 키를 OS 수준 암호화 저장소(Windows Credential Manager, macOS Keychain)에 저장합니다. Tauri 커맨드를 통해 키를 읽고 쓰는 방식으로 변경합니다.

**Expected Effect:**
- API 키 탈취 위험 원천 차단
- 사용자 계정 보안 강화
- 보안 점수 76 → 90+ 예상

---

### 🟡 중요 (P2)

#

#### Verification:

- Run: `cd vibereport-extension && pnpm compile`
- Run: `cd vibereport-extension && pnpm test`
- Confirm no compilation errors

**✅ After completing this prompt, proceed to [PROMPT-002]**

---

## 🟡 Priority 2 (High) - Execute Second

### [PROMPT-002] -1] E2E 테스트 스위트 추가

**⏱️ Execute this prompt now, then proceed to PROMPT-003**

> **🚨 REQUIRED: Use `replace_string_in_file` or `create_file` to make changes. Do NOT just show code.**

**Task**: -1] E2E Testing 스위트 추가

**Details:**

| 항목 | 내용 |
|:---|:---|
| **ID** | `TEST-001` |
| **Category** | 🧪 테스트 |
| **Complexity** | High |
| **Target Files** | `core/tests/e2e_test.rs` (신규), `core/tests/fixtures/` (신규) |

**Current State:** 186개 단위/통합 테스트가 있으나, 실제 파일 I/O부터 번역 API 모킹까지 전체 파이프라인을 검증하는 E2E 테스트가 없습니다.

**Improvement:** `wiremock` 크레이트로 AI API를 모킹하고, 다양한 형식의 샘플 파일을 사용하여 전체 번역 파이프라인을 검증하는 E2E 테스트 스위트를 구축합니다.

**Expected Effect:**
- 리팩토링 시 회귀 버그 방지
- 새 형식 추가 시 안정성 보장
- 테스트 커버리지 85 → 92+ 예상

---

#

#### Verification:

- Run: `cd vibereport-extension && pnpm compile`
- Run: `cd vibereport-extension && pnpm test`
- Confirm no compilation errors

**✅ After completing this prompt, proceed to [PROMPT-003]**

---

### [PROMPT-003] -2] UI 에러 피드백 상세화

**⏱️ Execute this prompt now, then proceed to PROMPT-004**

> **🚨 REQUIRED: Use `replace_string_in_file` or `create_file` to make changes. Do NOT just show code.**

**Task**: -2] UI 에러 피드백 상세화

**Details:**

| 항목 | 내용 |
|:---|:---|
| **ID** | `UI-001` |
| **Category** | 🎨 UI/UX |
| **Complexity** | Medium |
| **Target Files** | `apps/desktop/src/context/ToastStore.tsx`, `apps/desktop/src/lib/ipc.ts` |

**Current State:** 에러 발생 시 일반적인 Toast 알림만 표시됩니다. 에러 유형(네트워크, API 한도, 파일 형식 등)에 따른 구체적인 안내가 없습니다.

**Improvement:**
1. Rust 백엔드에서 구조화된 에러 타입 정의 (`AppError` enum)
2. 프론트엔드에서 에러 유형별 아이콘, 색상, 해결 방법 표시
3. API 한도 초과 시 남은 대기 시간 표시

**Expected Effect:**
- 사용자 문제 해결 시간 단축
- 지원 문의 감소
- UI/UX 점수 75 → 82+ 예상
<!-- AUTO-IMPROVEMENT-LIST-END -->

---

## 3. ✨ 기능 추가 항목 (새 기능)

<!-- AUTO-FEATURE-LIST-START -->

### 🟡 중요 (P2)

#

#### Verification:

- Run: `cd vibereport-extension && pnpm compile`
- Run: `cd vibereport-extension && pnpm test`
- Confirm no compilation errors

**✅ After completing this prompt, proceed to [PROMPT-004]**

---

### [PROMPT-004] -3] 번역 제외(ignore) 기능

**⏱️ Execute this prompt now, then proceed to PROMPT-005**

> **🚨 REQUIRED: Use `replace_string_in_file` or `create_file` to make changes. Do NOT just show code.**

**Task**: -3] 번역 제외(ignore) 기능

**Details:**

| 항목 | 내용 |
|:---|:---|
| **ID** | `FEAT-001` |
| **Category** | ✨ 기능 추가 |
| **Complexity** | Medium |
| **Target Files** | `core/src/config.rs`, `core/src/scanner.rs`, `apps/desktop/src/views/SettingsView.tsx` |

**Current State:** 스캔된 모든 파일이 번역 대상이 됩니다. 개발자 노트, 테스트 파일, 특정 언어 폴더 등을 제외할 방법이 없습니다.

**Improvement:**
1. `.modtranslatorignore` 파일 지원 (gitignore 형식)
2. `config.rs`에 `ignore_patterns: Vec<String>` 필드 추가
3. `scanner.rs`에서 패턴 매칭으로 파일 제외
4. 설정 UI에서 직접 패턴 편집 가능

**Expected Effect:**
- 불필요한 번역 작업 감소 → API 비용 절감
- 사용자 제어권 향상
- 대규모 모드팩 처리 시 효율성 증가

---

### 🟢 선택적 (P3)

#

#### Verification:

- Run: `cd vibereport-extension && pnpm compile`
- Run: `cd vibereport-extension && pnpm test`
- Confirm no compilation errors

**✅ After completing this prompt, proceed to [PROMPT-005]**

---

## 🟢 Priority 3 (Medium) - Execute Last

### [PROMPT-005] -1] 다국어 UI 지원 (i18n)

**⏱️ Execute this prompt now - FINAL PROMPT**

> **🚨 REQUIRED: Use `replace_string_in_file` or `create_file` to make changes. Do NOT just show code.**

**Task**: -1] 다국어 UI 지원 (i18n)

**Details:**

| 항목 | 내용 |
|:---|:---|
| **ID** | `FEAT-002` |
| **Category** | ✨ 기능 추가 |
| **Complexity** | Medium |
| **Target Files** | `apps/desktop/src/i18n/` (신규), `apps/desktop/src/App.tsx` |

**Current State:** UI 텍스트가 한국어로 하드코딩되어 있습니다. 글로벌 사용자 접근이 제한됩니다.

**Improvement:**
1. `react-i18next` 라이브러리 도입
2. `i18n/locales/` 폴더에 언어별 JSON 파일 생성 (ko, en, ja, zh)
3. 언어 선택 드롭다운 추가
4. 브라우저 언어 자동 감지

**Expected Effect:**
- 글로벌 사용자 접근성 향상
- 번역 프로젝트답게 다국어 지원
- 커뮤니티 번역 기여 가능
<!-- AUTO-FEATURE-LIST-END -->

#### Verification:

- Run: `cd vibereport-extension && pnpm compile`
- Run: `cd vibereport-extension && pnpm test`
- Confirm no compilation errors

**🎉 ALL PROMPTS COMPLETED! Run final verification.**

---


*Generated: 2025-12-01T15:27:24.555Z*