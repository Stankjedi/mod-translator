# Mod Translator 문서

Mod Translator 기술 문서에 오신 것을 환영합니다. 이 디렉토리에는 시스템 아키텍처, 검증 로직, 파일 형식 및 CLI 사용법에 대한 자세한 정보가 포함되어 있습니다.

## 📚 문서 목차

### [시스템 아키텍처 (System Architecture)](./ARCHITECTURE.md)
- 상위 수준 시스템 설계
- 핵심 구성 요소 (스캐너, 프로필, 보호기, 검증기)
- 번역 파이프라인
- 오류 처리 및 성능
- 향후 로드맵 및 강화 계획

### [검증 시스템 (Validation System)](./VALIDATION_SYSTEM.md)
- 토큰 보호 및 검증 로직에 대한 심층 분석
- 지원되는 토큰 유형 (범용 + 자리표시자)
- 자동 복구 메커니즘 (5단계 프로세스)
- 구성 및 오류 코드

### [지원되는 형식 (Supported Formats)](./FORMATS.md)
- 지원되는 파일 형식 상세 정보 (XML, JSON, YAML, Lua 등)
- 형식별 처리 및 검증
- 각 형식에 대한 토큰 보호 규칙

### [CLI 가이드 (CLI Guide)](./CLI_GUIDE.md)
- 명령줄 인터페이스 참조
- 구성 파일 (`.mod-translator.toml`)
- 고급 사용 예시 (일괄 처리, CI/CD)

### [보고서 (Reports)](./reports/)
- 감사 로그 및 QA 시나리오 보고서
