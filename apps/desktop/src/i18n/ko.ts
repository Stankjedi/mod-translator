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
  },
} as const

export type Dictionary = typeof dictionary

export function useI18n(): Dictionary {
  return dictionary
}
