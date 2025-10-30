export const dictionary = {
  common: {
    save: '저장',
    cancel: '취소',
    enabled: '사용',
    disabled: '사용 안 함',
  },
  settings: {
    steam: {
      title: 'Steam 연동',
      pathLabel: 'Steam 설치 경로',
      detect: '자동 감지',
      scan: '지정 경로 스캔',
      scanning: '스캔 중...',
      noteDetected: '감지된 경로: {path}',
      noteNotFound: 'Steam 설치 경로를 찾지 못했습니다. 직접 입력하세요.',
      noteEmpty: '경로가 비어 있습니다. 먼저 경로를 입력하거나 자동 감지를 사용하세요.',
      noteDone: '스캔을 완료했습니다. 워크숍 모드를 색인했습니다.',
      noteError: '스캔 중 오류가 발생했습니다. 경로와 권한을 확인하세요.',
    },
    limits: {
      title: '동시성 및 속도 제한',
      concurrency: '동시 작업자 수',
      workers: '작업자 수',
      bucket: '토큰 버킷 용량',
      refillMs: '토큰 충전 주기(ms)',
      hints: {
        concurrency: '동시에 처리할 번역 작업 수입니다.',
        workers: '병렬 처리를 담당하는 워커 수입니다.',
        bucket: '순간 폭주를 허용하는 최대 토큰입니다.',
        refillMs: '토큰이 다시 채워지는 주기입니다.',
      },
    },
    retry: {
      title: '재시도 정책',
      description: '429/503 응답에 대비해 지수 백오프와 보호 장치를 조정하세요.',
      fields: {
        maxRetries: {
          label: '최대 재시도 횟수',
          hint: '실패한 요청을 다시 시도할 최대 횟수입니다. 0이면 재시도하지 않습니다.',
        },
        initialDelayMs: {
          label: '초기 대기 시간(ms)',
          hint: '첫 재시도를 하기 전에 기다리는 시간입니다.',
        },
        multiplier: {
          label: '증가 배수',
          hint: '재시도할 때마다 대기 시간이 곱해지는 배수입니다.',
        },
        maxDelayMs: {
          label: '최대 대기 시간(ms)',
          hint: '대기 시간이 이 값을 초과하지 않도록 제한합니다.',
        },
      },
      toggles: {
        respectServerHints: {
          label: '서버 힌트 준수 (Retry-After)',
          hint: '서버가 Retry-After 헤더를 보낼 경우 해당 지시를 우선합니다.',
        },
        autoTune429: {
          label: '429 발생 시 동시성 자동 조정',
          hint: '429 응답이 반복되면 동시 작업 수를 줄여 제한을 완화합니다.',
        },
      },
    },
  },
} as const

export type Dictionary = typeof dictionary

export function useI18n(): Dictionary {
  return dictionary
}
