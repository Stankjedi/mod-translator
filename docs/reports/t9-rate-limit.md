# T9 QA 시나리오 & 계측 리포트

## 목적

- 429 응답과 서버 힌트가 수반되는 번역 지연 구간을 재현하고 회귀를 방지한다.
- 백엔드 스트리밍 이벤트에 계측 필드를 추가해 QA가 재시도/대기 동작을 검증할 수 있도록 한다.

## 준비 사항

- 데스크톱 앱을 `pnpm tauri:dev`로 실행한다.
- React 개발자 도구 또는 `window.__JOB_STORE__` 디버거 훅을 사용해 `JobStore.currentJob.metrics` 배열을 확인한다.
- 테스트용 번역 공급자는 모의 응답(429/Retry-After 헤더)을 반환하도록 HTTP 프록시를 구성한다.

## 시나리오별 기대 결과

### 1. 10라인 성공 후 429 → 19초 힌트 대기 → 재개 성공

1. 번역 요청 10건 처리 후 모의 서버가 `429`와 `Retry-After: 19` 헤더를 반환한다.
2. UI 로그에 `429 응답으로 서버 힌트에 따라 19.0초 대기 후 재시도합니다.` 메시지가 추가된다.
3. `currentJob.metrics` 배열에 아래 항목이 추가된다.
   - `status: "rate_limited"`
   - `errorCode: "RATE_LIMITED"`
   - `attempt: 1`
   - `usedServerHint: true`
   - `totalBackoffMs: 19000`
4. 19초 대기 후 재요청이 성공하면서
   - 진행 로그가 `번역 완료`로 갱신되고,
   - `metrics`에 `status: "success"`, `attempt: 2`, `totalBackoffMs: 19000` 엔트리가 추가된다.

### 2. 대기 중 취소

1. 429 재시도 대기 상태에서 취소 버튼을 누른다.
2. 백엔드가 즉시 `cancelRequested: true` 이벤트와 함께 `status: "canceled"`를 전송한다.
3. `metrics` 마지막 엔트리의 `status`가 `"canceled"`, `errorCode: "RATE_LIMITED"`, `attempt`가 취소 시점의 재시도 횟수로 기록된다.
4. UI 로그에 `사용자가 작업을 중단했습니다.` 메시지가 추가되고 작업이 종료된다.

### 3. 재시도 소진 후 Resume 성공

1. 서버가 연속해서 429를 반환하도록 설정한다.
2. 5회(`RATE_LIMIT_MAX_ATTEMPTS`) 모두 실패한 후 백엔드는 `status: "failed"`, `errorCode: "RATE_LIMITED"` 이벤트를 전송한다.
3. `metrics` 배열의 마지막 항목이 `attempt: 5`, `totalBackoffMs` 누적치, `status: "failed"`로 남는다.
4. 사용자가 동일한 작업을 다시 시작하면 `metrics`가 초기화되고 새 번역 세션이 성공적으로 완료된다.

## 계측 필드 검증

| 필드 | 설명 | 확인 방법 |
| --- | --- | --- |
| `provider` | 번역 공급자 식별자(`gemini`, `gpt` 등) | `currentJob.metrics[*].provider` |
| `modelId` | 사용한 모델 ID | 동일 |
| `status` | `rate_limited` / `success` / `failed` / `canceled` | 동일 |
| `errorCode` | `RATE_LIMITED`, `NETWORK_ERROR` 등 | 동일 |
| `attempt` | 1부터 시작하는 재시도 횟수 | 동일 |
| `usedServerHint` | 서버 힌트(`Retry-After` 등) 사용 여부 | 동일 |
| `totalBackoffMs` | 누적 대기 시간(ms) | 동일 |

## QA 체크리스트

| 번호 | 항목 | 결과 | 비고 |
| --- | --- | --- | --- |
| T9-1 | 429 힌트 기반 대기 후 자동 재시도 성공 | ✅ | `metrics`에 `rate_limited → success` 연속 기록 확인 |
| T9-2 | 대기 중 사용자 취소 시 즉시 중단 | ✅ | 취소 이벤트와 `metrics.status = "canceled"` 확인 |
| T9-3 | 재시도 소진 후 실패, Resume 시 계측 초기화 | ✅ | 실패 후 `metrics` 초기화, 재시작 성공 여부 확인 |

## 참고 로그 위치

- Rust 백엔드 이벤트: `core/src/jobs.rs` (`TranslationProgressEventPayload.metrics`)
- 프런트엔드 상태: `apps/desktop/src/context/JobStore.tsx` (`currentJob.metrics`)

리포트의 모든 항목이 ✅일 때 T9 QA 시나리오가 통과한 것으로 간주한다.
