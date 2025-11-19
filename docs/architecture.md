# Universal Mod Translator 아키텍처

## 개요

Universal Mod Translator는 코드 구조, 자리표시자 및 형식 토큰을 100% 보존하면서 스팀 창작마당 모드를 번역하도록 설계된 포괄적인 시스템입니다. 다양한 파일 형식과 게임별 규칙을 지원합니다.

## 핵심 원칙

1.  **100% 코드 보존**: 태그, 자리표시자, 이스케이프 및 엔티티는 정확히 보존되어야 합니다.
2.  **형식 불가지론 (Format-Agnostic)**: XML, JSON, YAML, PO, INI, CFG, CSV, Properties, Lua, TXT/Markdown을 지원합니다.
3.  **게임별 규칙**: 게임 감지 및 특정 규칙(RimWorld, Factorio 등)을 위한 플러그인 시스템입니다.
4.  **선택적 롤백**: 실패한 키는 원본으로 롤백하고, 성공한 키는 병합합니다.
5.  **대용량 파일 스트리밍**: 메모리 고갈 없이 수 메가바이트 파일을 처리합니다.
6.  **원자적 쓰기**: 원본을 백업하고, 임시 파일에 쓴 다음 교체(swap)합니다.

## 시스템 구성 요소

### 1. 형식 처리기 (`core/src/formats/`)
각 형식에는 `FormatHandler` 특성(trait)을 구현하는 처리기가 있습니다:
-   `extract()`: 번역 가능한 키-값 쌍을 추출합니다.
-   `merge()`: 구조를 보존하면서 번역을 다시 삽입합니다.
-   **구현됨**: JSON, INI/CFG.
-   **스텁 (Stubs)**: XML, YAML, PO, CSV, Properties, Lua, TXT.

### 2. 파일 스캐너 (`core/src/scanner.rs`)
구성 가능한 규칙으로 모드 디렉토리를 스캔합니다:
-   **포함**: `Languages/`, `locale/`, `i18n/` 등.
-   **제외**: 바이너리 파일 (`.dll`, `.png`), 대용량 파일 (>20MB).

### 3. 게임 프로필 (`core/src/profiles/`)
게임별 규칙을 위한 플러그인 시스템:
-   **RimWorld**: `About/About.xml` 감지, `{PAWN_*}` 보호.
-   **Factorio**: `info.json` 감지, `__ENTITY__` 보호, `locale/*.cfg` 사용.
-   **Stardew Valley**: `manifest.json` 감지, `i18n/*.json` 사용.
-   **Generic**: 인식되지 않는 모드에 대한 대체(Fallback).

### 4. 보호 시스템 (`core/src/protector.rs`)
번역 전에 보호된 토큰을 마커(`⟦MT:PLACEHOLDER:0⟧`)로 대체합니다.
-   **보호된 유형**: 태그, 자리표시자 (`{0}`, `%s`), ICU MessageFormat, Mustache, 서식 있는 텍스트(Rich Text), 엔티티, 이스케이프.

### 5. 검증 시스템 (`core/src/validator.rs`)
품질 보장을 위한 다중 게이트 검증입니다. 자세한 내용은 [검증 시스템](./VALIDATION_SYSTEM.md)을 참조하세요.

### 6. 인코딩 보존 (`core/src/encoding.rs`)
-   UTF-8 (BOM 포함), UTF-16 LE/BE, Latin-1을 감지하고 보존합니다.
-   줄 바꿈 스타일(LF vs CRLF)을 보존합니다.

## 번역 파이프라인

1.  **스캔 (Scan)**: 번역 가능한 파일 찾기.
2.  **감지 (Detect)**: 게임 프로필 및 파일 형식 식별.
3.  **로드 (Load)**: 인코딩 감지와 함께 파일 읽기.
4.  **추출 (Extract)**: 번역 가능한 항목 추출.
5.  **보호 (Protect)**: 모든 보호된 토큰 마스킹.
6.  **번역 (Translate)**: 마스킹된 텍스트를 AI 제공자에게 전송.
7.  **검증 (Validate)**: 모든 검증 게이트 확인.
8.  **복원 (Restore)**: 토큰 마스킹 해제.
9.  **병합 (Merge)**: 번역을 원본 구조에 다시 삽입.
10. **쓰기 (Write)**: 원본 인코딩/줄 바꿈 스타일로 저장.

## 오류 처리

1.  **파싱 오류**: 파일 건너뛰기, 오류 로깅.
2.  **검증 오류**: 해당 키에 대해 원본으로 롤백, 1회 재시도.
3.  **API 오류**: 지수 백오프(Exponential backoff), 재개.
4.  **IO 오류**: 백업에서 복원.

## 프로덕션 강화 계획

### 완료됨
-   **경로 및 직렬화**: Tauri 명령을 위한 향상된 `Serialize`/`Deserialize`. 경로 처리 개선.
-   **시간 처리**: 통일된 시간 형식.
-   **작업 로그**: 작업 요약을 위한 JSONL 로깅.
-   **Tauri 2.x 구성**: 런타임 제네릭 및 MSI 아이콘 경로 수정.
-   **UI/Core 계약**: TypeScript 타입을 Rust 구조체와 일치시킴.

### 향후 작업
-   **실제 번역 파이프라인**: DLL 문자열 추출, ZIP 재패키징 구현.
-   **작업 실행기 (Job Executor)**: 취소 및 진행 상황 업데이트가 포함된 비동기 작업 스케줄러 추가.
-   **로그 회전**: 로그 회전 및 복구 UI 구현.
-   **보안 저장소**: API 키를 위해 Windows 자격 증명 관리자 / macOS 키체인과 통합.
-   **백업 전략**: 번역 전 전체 디렉토리 백업.
-   **인코딩 왕복 (Roundtrip)**: 견고한 `chardetng` 통합.
-   **속도 제한**: 제공자별 기능 및 스마트 스로틀링.
