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
| 1 | PROMPT-001 | [P2-1] Add Japanese & Chinese UI Support | P2 | ⬜ Pending |
| 2 | PROMPT-002 | [P2-2] Generate API Documentation (rustdoc) | P2 | ⬜ Pending |
| 3 | PROMPT-003 | [P2-3] Large File Streaming Processing | P2 | ⬜ Pending |
| 4 | PROMPT-004 | [P2-4] Complete LUA Handler Merge Function | P2 | ⬜ Pending |
| 5 | PROMPT-005 | [P3-1] User Onboarding/Tutorial | P3 | ⬜ Pending |
| 6 | PROMPT-006 | [P3-2] Glossary Feature | P3 | ⬜ Pending |

**Total: 6 prompts** | **Completed: 0** | **Remaining: 6**

---

## 🟡 Priority 2 (Important) - Execute First

### [PROMPT-001] [P2-1] Add Japanese & Chinese UI Support

**⏱️ Execute this prompt now, then proceed to PROMPT-002**

> **🚨 REQUIRED: Use `replace_string_in_file` or `create_file` to make changes. Do NOT just show code.**

**Task**: Add Japanese and Chinese (Simplified) language support to the existing i18n system

**Details:**

| Item | Content |
|:---|:---|
| **ID** | `I18N-001` |
| **Category** | 🌐 i18n |
| **Complexity** | Low |
| **Target Files** | `apps/desktop/src/i18n/locales/ja.ts`, `apps/desktop/src/i18n/locales/zh.ts`, `apps/desktop/src/i18n/types.ts`, `apps/desktop/src/i18n/index.ts` |

**Current State:** v0.1.3 implemented Korean/English UI with react-i18next. The `SupportedLocale` type is defined as `"ko" | "en"` only.

**Implementation Steps:**
1. Update `apps/desktop/src/i18n/types.ts` to add 'ja' and 'zh' to `SupportedLocale` type
2. Create `apps/desktop/src/i18n/locales/ja.ts` with Japanese translations (copy structure from `en.ts`)
3. Create `apps/desktop/src/i18n/locales/zh.ts` with Simplified Chinese translations
4. Update `apps/desktop/src/i18n/index.ts` to import and register new language resources
5. Ensure language dropdown in settings shows all 4 languages

**Expected Result:**
- Users can select Japanese or Chinese from the language dropdown
- All UI text is translated appropriately
- Browser language auto-detection includes ja/zh

#### Verification:
- Run: `cd apps/desktop && pnpm typecheck`
- Confirm no TypeScript errors

**✅ After completing this prompt, proceed to [PROMPT-002]**

---

### [PROMPT-002] [P2-2] Generate API Documentation (rustdoc)

**⏱️ Execute this prompt now, then proceed to PROMPT-003**

> **🚨 REQUIRED: Use `replace_string_in_file` or `create_file` to make changes. Do NOT just show code.**

**Task**: Add rustdoc comments to public APIs and configure CI for documentation generation

**Details:**

| Item | Content |
|:---|:---|
| **ID** | `DOC-001` |
| **Category** | 📚 Documentation |
| **Complexity** | Medium |
| **Target Files** | `core/src/lib.rs`, `core/Cargo.toml`, `.github/workflows/docs.yml` |

**Current State:** `core/src/lib.rs` exports 30+ public modules and types without `///` doc comments. Running `cargo doc` produces empty documentation.

**Implementation Steps:**
1. Add crate-level documentation at the top of `core/src/lib.rs` using `//!` comments
2. Add `///` documentation comments to all `pub use` exports explaining each type/function
3. Update `core/Cargo.toml` with documentation metadata:
   - `documentation = "https://stankjedi.github.io/mod-translator"`
   - `homepage = "https://github.com/Stankjedi/mod-translator"`
4. Create `.github/workflows/docs.yml` for automatic doc generation on push to main
5. Add docs badge to README.md

**Expected Result:**
- `cargo doc --open` generates comprehensive HTML documentation
- GitHub Actions automatically deploys docs on push to main

#### Verification:
- Run: `cd core && cargo doc --no-deps`
- Confirm documentation generates without warnings

**✅ After completing this prompt, proceed to [PROMPT-003]**

---

### [PROMPT-003] [P2-3] Large File Streaming Processing

**⏱️ Execute this prompt now, then proceed to PROMPT-004**

> **🚨 REQUIRED: Use `replace_string_in_file` or `create_file` to make changes. Do NOT just show code.**

**Task**: Implement streaming processing for large files to reduce memory usage

**Details:**

| Item | Content |
|:---|:---|
| **ID** | `PERF-001` |
| **Category** | ⚡ Performance |
| **Complexity** | High |
| **Target Files** | `core/src/formats/mod.rs`, `core/src/formats/json.rs`, `core/src/pipeline.rs` |

**Current State:** Files are loaded entirely into memory before processing. Large modpacks (10MB+) can cause memory issues.

**Implementation Steps:**
1. Add `is_large_file(path: &Path) -> bool` helper function in `core/src/formats/mod.rs` (threshold: 5MB)
2. Add `StreamingConfig` struct with chunk size and threshold settings
3. Implement chunked JSON parsing using `serde_json::StreamDeserializer` for large files
4. Update `FormatHandler` trait with optional `extract_streaming()` method
5. Update pipeline to detect large files and use streaming mode automatically
6. Update progress reporting to show chunk-level progress

**Expected Result:**
- Files under 5MB: processed normally (current behavior preserved)
- Files over 5MB: processed in chunks (memory efficient)
- Memory usage reduced by up to 80% for large files
- Progress bar shows chunk completion percentage

#### Verification:
- Run: `cd core && cargo test`
- Confirm all 199 tests pass

**✅ After completing this prompt, proceed to [PROMPT-004]**

---

### [PROMPT-004] [P2-4] Complete LUA Handler Merge Function

**⏱️ Execute this prompt now, then proceed to PROMPT-005**

> **🚨 REQUIRED: Use `replace_string_in_file` or `create_file` to make changes. Do NOT just show code.**

**Task**: Complete the stub implementation of LUA format handler's merge function

**Details:**

| Item | Content |
|:---|:---|
| **ID** | `CODE-001` |
| **Category** | 🔧 Code Quality |
| **Complexity** | Medium |
| **Target Files** | `core/src/formats/lua.rs` |

**Current State:** The evaluation report noted "일부 LUA 핸들러 merge 기능 stub 상태". The `FormatHandler` trait's `merge` method for LUA files is incomplete.

**Implementation Steps:**
1. Review current `LuaHandler` implementation in `core/src/formats/lua.rs`
2. Implement proper merge logic that:
   - Preserves original LUA file structure and comments
   - Replaces only the translated string values
   - Handles nested table structures correctly
   - Maintains Lua syntax validity
3. Add unit tests for merge functionality:
   - Test simple key-value pairs
   - Test nested tables
   - Test multiline strings
   - Test comments preservation
4. Update any related error handling

**Expected Result:**
- LUA files can be properly merged after translation
- Original file structure and formatting preserved
- All existing tests continue to pass
- New merge tests added and passing

#### Verification:
- Run: `cd core && cargo test lua`
- Confirm all LUA-related tests pass

**✅ After completing this prompt, proceed to [PROMPT-005]**

---

## 🟢 Priority 3 (Nice-to-have) - Execute Last

### [PROMPT-005] [P3-1] User Onboarding/Tutorial

**⏱️ Execute this prompt now, then proceed to PROMPT-006**

> **🚨 REQUIRED: Use `replace_string_in_file` or `create_file` to make changes. Do NOT just show code.**

**Task**: Implement first-run onboarding experience for new users

**Details:**

| Item | Content |
|:---|:---|
| **ID** | `UX-001` |
| **Category** | 🎨 UI/UX |
| **Complexity** | Medium |
| **Target Files** | `apps/desktop/src/views/OnboardingView.tsx`, `apps/desktop/src/stores/appStore.ts`, `apps/desktop/src/App.tsx`, `apps/desktop/src/i18n/locales/*.ts` |

**Current State:** No guidance for first-time users. Users must figure out API key setup and mod scanning on their own.

**Implementation Steps:**
1. Create `apps/desktop/src/views/OnboardingView.tsx` with 4-step wizard:
   - Step 1: Welcome message + Language selection
   - Step 2: API Key setup with links to provider documentation
   - Step 3: Steam/Mod folder detection and selection
   - Step 4: Quick start guide showing basic workflow
2. Add `hasCompletedOnboarding: boolean` field to app store (persisted via Stronghold)
3. Check onboarding status on app launch in `App.tsx`, redirect if not completed
4. Add "Show Onboarding Again" option in Settings view
5. Add all onboarding text to i18n translation files (ko, en, ja, zh)

**Expected Result:**
- First-time users see guided setup wizard automatically
- "Skip" and "Don't show again" options available
- Settings menu allows re-running onboarding
- All text properly internationalized

#### Verification:
- Run: `cd apps/desktop && pnpm typecheck`
- Confirm no TypeScript errors

**✅ After completing this prompt, proceed to [PROMPT-006]**

---

### [PROMPT-006] [P3-2] Glossary Feature

**⏱️ Execute this prompt now - FINAL PROMPT**

> **🚨 REQUIRED: Use `replace_string_in_file` or `create_file` to make changes. Do NOT just show code.**

**Task**: Implement glossary system for consistent translation of game-specific terms

**Details:**

| Item | Content |
|:---|:---|
| **ID** | `FEAT-001` |
| **Category** | ✨ Feature Addition |
| **Complexity** | High |
| **Target Files** | `core/src/glossary.rs`, `core/src/lib.rs`, `core/src/pipeline.rs`, `apps/desktop/src/views/GlossaryView.tsx` |

**Current State:** Game-specific terms (item names, place names, character names) are translated inconsistently. No mechanism to enforce community-agreed translations.

**Implementation Steps:**
1. Create `core/src/glossary.rs` module with:
   - `GlossaryEntry` struct: source term, target translation, optional context/notes
   - `Glossary` struct: collection of entries with game profile association
   - `load_from_file(path: &Path) -> Result<Glossary>` for JSON file loading
   - `apply_to_text(&self, text: &str) -> String` for pre-translation replacement
   - `save_to_file(&self, path: &Path) -> Result<()>` for exporting
2. Add `pub mod glossary;` to `core/src/lib.rs` and re-export public types
3. Integrate glossary into translation pipeline:
   - Load glossary before translation
   - Apply glossary replacements to source text
   - Protect glossary terms like placeholders
4. Create `apps/desktop/src/views/GlossaryView.tsx`:
   - List view of all glossary entries
   - Add/Edit/Delete entry forms
   - Import from JSON file
   - Export to JSON file
   - Filter by game profile
5. Add navigation link to Glossary view in sidebar
6. Add glossary-related i18n keys to all locale files

**Expected Result:**
- Users can create and manage glossary entries in UI
- Glossary terms are consistently applied before AI translation
- Import/export allows sharing glossaries with community
- Game-specific terms maintain consistency across translations

#### Verification:
- Run: `cd core && cargo test glossary`
- Run: `cd apps/desktop && pnpm typecheck`
- Confirm all tests pass

**🎉 ALL PROMPTS COMPLETED! Run final verification:**

```bash
cd core && cargo test --workspace
cd apps/desktop && pnpm typecheck
```

---

*Generated: 2025-12-03*