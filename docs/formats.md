# 지원되는 파일 형식 (Supported File Formats)

Mod Translator는 게임 모딩에서 일반적으로 사용되는 다양한 파일 형식을 지원합니다. 각 형식은 구조를 보존하면서 추출 및 병합을 처리합니다.

## 개요

| 형식 | 확장자 | 상태 | 처리기 (Handler) |
| :--- | :--- | :--- | :--- |
| XML | `.xml` | ✅ 전체 지원 | XmlHandler |
| JSON | `.json`, `.jsonl` | ✅ 전체 지원 | JsonHandler |
| YAML | `.yaml`, `.yml` | ✅ 전체 지원 | YamlHandler |
| PO (gettext) | `.po`, `.pot` | ✅ 전체 지원 | PoHandler |
| INI/CFG | `.ini`, `.cfg` | ✅ 전체 지원 | IniHandler |
| CSV | `.csv`, `.tsv` | ✅ 전체 지원 | CsvHandler |
| Properties | `.properties` | ✅ 전체 지원 | PropertiesHandler |
| Lua | `.lua` | ✅ 전체 지원 | LuaHandler |
| Markdown | `.md` | ✅ 전체 지원 | MarkdownHandler |
| Plain Text | `.txt` | ✅ 전체 지원 | TxtHandler |

## 형식 상세 정보

### Markdown
-   **번역 대상**: 문단, 헤더, 목록, 인용문.
-   **보호 대상**: 코드 블록, 인라인 코드, 링크 (`[text](url)`), 이미지, 수식, HTML 태그.
-   **검증**: 균형 잡힌 코드 펜스(code fences).

### Properties (Java .properties)
-   **번역 대상**: `key=value` 또는 `key:value`의 값.
-   **보호 대상**: 키, 주석 (`#`, `!`), 유니코드 이스케이프 (`\uXXXX`), 형식 토큰.

### Lua
-   **번역 대상**: 문자열 리터럴 (작은/큰 따옴표, 긴 대괄호 `[[...]]`).
-   **보호 대상**: 변수 이름, 테이블 키, 주석 (`--`), 이스케이프.

## 토큰 보호
모든 형식은 통합된 **보호기 시스템 (Protector System)**을 사용합니다:
1.  번역 불가능한 요소 식별 (형식별).
2.  `⟦MT:TYPE:N⟧`으로 대체.
3.  마스킹된 텍스트 번역.
4.  토큰 복원.

## 형식 감지
1.  **확장자**: 기본 방법 (예: `.md` -> Markdown).
2.  **콘텐츠 서명**: 대체 방법 (예: `local` + `return` -> Lua).
3.  **수동 재정의**: CLI 또는 구성을 통해.
