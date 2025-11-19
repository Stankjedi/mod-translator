# CLI 사용자 가이드

## 설치

```bash
cargo build --release
# 바이너리는 target/release/mod-translator 에 위치합니다
```

## 기본 사용법

```bash
# 모드 디렉토리 번역
mod-translator translate /path/to/mod --target-lang ko

# 특정 게임 프로필 사용
mod-translator translate /path/to/mod --profile rimworld --target-lang ko

# 드라이 런 (변경 없이 미리보기)
mod-translator translate /path/to/mod --dry-run --target-lang ko
```

## 명령 참조

### `translate`
모드 디렉토리의 파일을 번역합니다.

**옵션:**
-   `--profile <PROFILE>`: `auto` (기본값), `rimworld`, `factorio`, `stardew`, `generic`.
-   `--target-lang <LANG>`: 대상 언어 코드 (예: `ko`, `ja`, `es`).
-   `--mode <MODE>`: `strict` (기본값) 또는 `lenient`.
-   `--include <PATTERNS>`: 포함할 Glob 패턴.
-   `--exclude <PATTERNS>`: 제외할 Glob 패턴.
-   `--encoding <ENCODING>`: 인코딩 강제 (`utf-8`, `latin1` 등).
-   `--dry-run`: 변경 사항 미리보기.
-   `--report <PATH>`: 상세 JSON 보고서 출력.
-   `--provider <PROVIDER>`: AI 제공자 (`gemini`, `gpt`, `claude`).
-   `--api-key <KEY>`: API 키 (환경 변수 `TRANSLATOR_API_KEY` 권장).

## 구성 파일
모드 디렉토리 또는 홈 디렉토리에 `.mod-translator.toml`을 생성합니다.

```toml
[default]
profile = "auto"
mode = "strict"

[scan]
include = ["**/*.xml", "**/*.json"]
exclude = ["**/*.dll", "**/*.png"]
max-file-size = 20971520

[validation]
check-placeholders = true
max-length-multiplier = 4

[api]
provider = "gemini"
model = "gemini-2.5-flash"
```

## 환경 변수
-   `TRANSLATOR_API_KEY`: API 키.
-   `TRANSLATOR_CONFIG`: 구성 파일 경로.
-   `TRANSLATOR_LOG_LEVEL`: 로그 레벨.

## 문제 해결
-   **"Unsupported format" (지원되지 않는 형식)**: 파일 확장자 또는 바이너리 임계값을 확인하세요.
-   **"PLACEHOLDER_MISMATCH" (자리표시자 불일치)**: 번역에서 토큰이 누락되었습니다. `--mode lenient`를 시도해보세요.
-   **"Rate limit exceeded" (속도 제한 초과)**: 동시성을 줄이거나 `--max-retry`를 사용하세요.
