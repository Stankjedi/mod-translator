# 검증 시스템 (Validation System)

## 개요

검증 시스템은 번역 중에 보호된 토큰, 자리표시자 및 형식 구조가 보존되도록 보장합니다. 일반적인 LLM 번역 오류를 처리하기 위해 자동 복구 기능이 있는 다중 전략 접근 방식을 사용합니다.

## 핵심 구성 요소

1.  **PlaceholderValidator** (`core/src/placeholder_validator.rs`): 다중 집합(multiset) 비교를 사용하는 메인 엔진.
2.  **Protector** (`core/src/protector.rs`): 토큰 감지 및 마스킹.
3.  **ValidatorConfig**: 엄격성, 재시도 및 자동 수정에 대한 구성.
4.  **ValidationLogger**: JSONL 로깅 및 실시간 지표.

## 지원되는 토큰 유형

### 형식 토큰
-   **PRINTF**: `%s`, `%1$s`, `%0.2f` (Minecraft, Factorio)
-   **DOTNET**: `{0}`, `{1:0.##}` (RimWorld, Cities:Skylines)
-   **NAMED**: `{name}`, `{PAWN_label}` (RimWorld)
-   **SHELL**: `$VAR`, `${count}`
-   **FACTORIO**: `__1__`, `__ENTITY__iron-ore__`
-   **ICU**: `{count, plural, ...}`

### 마크업 및 서식 있는 텍스트
-   **TAG**: `<tag>`, `</tag>`
-   **BBCODE**: `[b]`, `[color=#ff0000]`
-   **RWCOLOR**: `<color=#fff>` (RimWorld)
-   **MCCOLOR**: `§a`, `§l` (Minecraft)
-   **RICHTEXT**: `<sprite=icon>` (Unity)

### 기타
-   **MATHEXPR**: `3.14 × r^2`
-   **RANGE**: `10-20`
-   **UNIT**: `16 ms`, `60 FPS`
-   **ESCAPE**: `\n`, `\t`

## 검증 파이프라인

1.  **전처리**: 텍스트 추출, 토큰 마스킹 (`⟦MT:TAG:0⟧`), 형식 토큰 감지 (`{n}`).
2.  **번역**: LLM이 마스킹된 텍스트를 번역합니다.
3.  **후처리**: 번역에서 토큰을 파싱합니다.
4.  **검증**:
    -   다중 집합 비교 (개수 확인).
    -   순서 보존 확인.
5.  **자동 복구** (검증 실패 시):
    -   **1단계: 누락된 토큰 재주입**: 상대적 위치에 누락된 토큰 추가 (성공률 ~85%).
    -   **2단계: 쌍 균형 맞추기**: 불균형 태그 수정 (성공률 ~90%).
    -   **3단계: 초과분 제거**: 예상치 못한 토큰 제거 (성공률 ~95%).
    -   **4단계: 형식 수정**: 누락된 `{n}` 토큰 수정 (성공률 ~80%).
    -   **5단계: 바인딩 보존**: `{n}%` 패턴이 함께 유지되도록 보장 (성공률 ~99%).
6.  **재시도**: 복구 실패 시 더 엄격한 프롬프트로 재시도 (최대 1회).

## 오류 코드

| 코드 | 설명 | 자동 복구 전략 |
| :--- | :--- | :--- |
| `PLACEHOLDER_MISMATCH` | 토큰 수 불일치 | 재주입 + 균형 맞추기 + 제거 |
| `PAIR_UNBALANCED` | 불균형 태그 | 누락된 닫기 태그 추가 |
| `FORMAT_TOKEN_MISSING` | `{n}` 누락 | 상대적 위치에 재주입 |
| `XML_MALFORMED_AFTER_RESTORE` | 깨진 XML 구조 | 복구 불가 |
| `FACTORIO_ORDER_ERROR` | `__n__` 순서 불일치 | 순서 복원 |

## 구성

```yaml
validator:
  enable_autofix: true
  retry_on_fail: true
  retry_limit: 1
  strict_pairing: true
  preserve_percent_binding: true
  jsonl_logging: true
```

## 사용 예시 (Rust)

```rust
use mod_translator_core::{PlaceholderValidator, Segment};

let validator = PlaceholderValidator::with_default_config();
let segment = Segment::new(file, line, key, original, preprocessed);

match validator.validate(&segment, translated_text) {
    Ok(recovered) => println!("Valid: {}", recovered),
    Err(report) => eprintln!("Error: {:?}", report.code),
}
```
