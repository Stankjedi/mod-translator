# Production Hardening Plan

이 문서는 Windows용 배포를 염두에 둔 mod-translator 워크스페이스의 하드닝 전략을 요약합니다.

## Patch Plan 요약

1. **경로 및 직렬화 안정성 강화**  
   - 모든 Tauri 명령 입출력 구조체가 `Serialize`/`Deserialize`/`Clone`/`Debug`를 구현하도록 정비했습니다.  
   - UTF-8 변환 실패 시 사용자에게 의미 있는 오류를 돌려보내도록 `steam.rs`와 `library.rs`의 경로 처리기를 개선했습니다.
2. **시간 처리 일원화**  
   - `core::time::format_system_time` 유틸리티를 도입해 `chrono` 경고를 제거하고 Tauri 경계를 넘나들 수 있는 구조체(`FormattedTimestamp`)를 추가했습니다.
3. **작업 로그 보존**  
   - `jobs.rs`는 각 번역 작업 요약을 JSONL로 디스크에 기록하여 예기치 않은 종료 시 상태 복원을 지원합니다.
4. **Tauri 2.x 구성 정비**  
   - 런타임 제네릭을 `tauri::Builder::<tauri::Wry>`로 고정하고 MSI 아이콘 경로를 명시해 Windows 배포를 안정화했습니다.
5. **UI ↔ 코어 계약 정렬**  
   - `apps/desktop/src/types/core.ts`에 Rust 구조체와 1:1로 매핑되는 TypeScript 타입을 정의하고 컨텍스트/뷰를 갱신했습니다.

## 남은 작업 제안

- **실제 번역 파이프라인 구현**: DLL 문자열 추출, 텍스트 인코딩 감지, ZIP 재패키징 로직을 `core::library` 및 별도 파이프라인 모듈로 확장합니다.
- **작업 실행기**: 현재는 미리보기만 반환하므로 Tokio 작업 스케줄러와 채널을 추가해 비동기 처리, 취소, 진행률 업데이트를 구현합니다.
- **로그 회전 및 복구**: JSONL 로그를 읽어 UI에 복구 옵션을 제공하고, 크기 제한/회전을 도입합니다.
- **보안 저장소 통합**: Windows 자격 증명 관리자나 macOS Keychain과 연동해 API 키를 안전하게 보관합니다.

## 제안된 전략

### 백업 전략

- 번역 실행 전 `workshop/content/<app>/<mod>` 디렉터리를 타임스탬프가 포함된 백업 디렉터리(예: `backups/<mod>/<YYYYMMDD-hhmmss>`)
  로 통째로 복제합니다.
- 대용량 ZIP/7z 아카이브는 원본 파일을 별도 `archives/` 경로에 복사하고, 번역 결과는 새 아카이브를 생성합니다. 원본 아카이브는
  절대 덮어쓰지 않습니다.
- 백업 완료 후 UI에 백업 경로를 전달해 사용자가 롤백할 수 있도록 합니다.

### 텍스트 인코딩 감지 및 라운드트립

- `chardetng` 또는 `encoding_rs` 기반 감지를 사용해 입력 텍스트를 UTF-8로 변환합니다.
- 감지 결과와 함께 신뢰도를 기록하고, 신뢰도가 낮을 때는 사용자의 확인을 요구합니다.
- 쓰기 시에는 원본 인코딩으로 재인코딩하되, 실패 시 UTF-8로 저장하고 백업 경로를 안내합니다.

### 길이 제한 및 재시도 정책

- 공급자별 요청 제한을 정의(`TranslatorProviderCapabilities`)하고, 입력 문자열을 토큰 수 추정치를 기준으로 청크 분할합니다.
- 429/503 등의 속도 제한 응답 시 지수 백오프(예: 1s, 2s, 4s)와 지터를 적용하며 최대 재시도 횟수를 구성 옵션으로 노출합니다.
- 청크별 성공/실패를 추적해 UI에 세부 진행률을 보고하고, 실패한 청크는 재시도 큐에 삽입합니다.

### 최소 로그 및 복구

- 현재 JSONL 로그를 기반으로 `jobs.rs`에 `load_recent_jobs()` 헬퍼를 추가해 UI가 앱 시작 시 마지막 N개 작업을 복원할 수 있게 합니다.
- 치명적인 실패 메시지는 로그에도 포함해 사용자 지원을 위한 진단 정보를 확보합니다.
- 향후에는 로그 경로를 설정 화면에서 커스터마이즈할 수 있도록 하되, 기본 경로는 OS별 `data_dir/mod-translator/logs`를 유지합니다.

## 참고 링크

- [Tauri 2.x Configuration](https://tauri.app/v2/api/config/)
- [chrono crate](https://docs.rs/chrono/latest/chrono/)
- [encoding_rs](https://docs.rs/encoding_rs/latest/encoding_rs/)
