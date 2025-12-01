import type { TranslationDictionary } from "./types";

export const ko: TranslationDictionary = {
  common: {
    save: "저장",
    cancel: "취소",
    enabled: "활성",
    disabled: "비활성",
    run: "실행",
    preparing: "준비 중...",
    loading: "로딩 중...",
    error: "오류",
    success: "성공",
    warning: "경고",
    info: "정보",
    confirm: "확인",
    close: "닫기",
    delete: "삭제",
    edit: "편집",
    add: "추가",
    search: "검색",
    filter: "필터",
    refresh: "새로고침",
    back: "뒤로",
    next: "다음",
    previous: "이전",
    done: "완료",
    retry: "재시도",
    noData: "데이터가 없습니다.",
    language: "언어",
  },

  navigation: {
    dashboard: "대시보드",
    dashboardDesc: "최근 상태와 진행 현황 요약",
    mods: "모드 관리",
    modsDesc: "설치된 워크샵 콘텐츠 살펴보기",
    progress: "진행 상황",
    progressDesc: "번역 파이프라인 모니터링",
    settings: "설정",
    settingsDesc: "번역기 환경과 스캔 옵션 구성",
  },

  app: {
    title: "모드 번역기",
    controlCenter: "제어 센터",
    description:
      "스팀 라이브러리를 점검하고 번역 작업을 실행하며 프로젝트 전반을 한눈에 살펴보세요.",
    workspace: "Mod Translator 작업 공간",
    workspaceDesc:
      "스팀과 연결하고 AI 번역기를 조율하며 품질을 모니터링하세요.",
    policyAcknowledged: "정책 동의가 기록되었습니다.",
    policyRequired: "정책 안내를 확인하고 동의해 주세요.",
  },

  dashboard: {
    title: "오늘의 요약",
    description:
      "정책 동의 상태와 라이브러리 스캔 결과, 워크샵 경고를 한 자리에서 확인할 수 있습니다.",
    summaryTitle: "오늘의 요약",
    summaryDesc:
      "정책 동의 상태와 라이브러리 스캔 결과, 워크샵 경고를 한 자리에서 확인할 수 있습니다.",
    detectedLibraries: "감지된 라이브러리",
    scanning: "스캔 중",
    healthyPaths: "정상 경로 {{count}}개",
    foundMods: "발견된 모드",
    workshopRoots: "워크샵 루트 {{count}}개",
    noWorkshopRoots: "워크샵 루트를 찾지 못했습니다",
    warnings: "주의 항목",
    noWarnings: "추가 조치가 필요한 경고가 없습니다.",
    reviewWarnings: "경고를 확인하고 필요한 작업을 진행하세요.",
    highlights: {
      libraryScan: "라이브러리 스캔 결과",
      workshopPath: "워크샵 경로",
      jobStatus: "번역 작업 현황",
    },
    notes: {
      scanningWorkshop: "워크샵 콘텐츠를 찾는 중입니다.",
      waitingForSteam: "스팀 경로가 확인되면 자동으로 스캔이 실행됩니다.",
      librariesDetected: "라이브러리가 정상적으로 감지되었습니다.",
      workshopConnected: "워크샵 콘텐츠가 연결된 라이브러리를 찾았습니다.",
      workshopNotFound:
        "워크샵 경로를 찾지 못했습니다. Steam을 한 번 실행한 뒤 다시 시도하세요.",
    },
    job: {
      noJobs: "예약된 번역 작업이 없습니다.",
      addJobHint: "모드 관리 탭에서 새 작업을 추가해 보세요.",
      recentCompleted: "최근 완료 {{count}}건",
      failed: "실패 {{count}}건",
      running: "실행 중 {{count}}건",
      pending: "준비 중 {{count}}건",
      queued: "대기 {{count}}건",
      completed: "완료 {{count}}건",
    },
    quickActions: {
      rescan: "라이브러리 다시 스캔",
      rescanDesc:
        "Steam 경로의 libraryfolders.vdf를 다시 읽어서 모드 목록을 갱신합니다.",
      viewJob: "{{modName}} 작업 보기",
      scheduleJob: "번역 작업 예약",
      checkQueue: "대기 중 작업 확인",
      qualityGuard: "품질 가드 설정",
      qualityGuardDesc: "추후 번역 품질 검증 도구와 연동될 예정입니다.",
    },
    gameSummary: {
      title: "게임별 모드 분포",
      noData: "스캔된 모드가 없어 분포 정보를 표시할 수 없습니다.",
      modsDetected: "감지된 모드 {{count}}개",
      warnings: "경고 {{count}}건",
    },
    pipeline: {
      title: "파이프라인 스냅샷",
      stages: [
        "워크샵 압축 해제",
        "파일 형식 식별 및 텍스트 자산 분류",
        "JSON/INI/XML/RESX 리소스 파싱",
        "플레이스홀더 고정 후 번역 실행",
        "플레이스홀더와 마크업 검증",
        "리소스 재패키징 또는 패치 생성",
      ],
    },
    debug: {
      title: "디버그: 라이브러리 탐지 파이프라인",
      description:
        "정규화된 경로와 중복/심볼릭 링크 건너뛰기 정보를 빠르게 점검할 수 있습니다.",
      candidates: "감지된 경로 후보",
      finalLibraries: "최종 라이브러리 세트",
      noCandidates: "경로 후보가 비어 있습니다.",
      noFinalLibraries: "최종 라이브러리가 비어 있습니다.",
      original: "원본",
      canonical: "정규화 경로",
      key: "키",
      status: "상태",
      note: "메모",
      noCanonicalPaths: "정규화된 경로가 없습니다.",
      rejectedCandidates: "제외된 후보",
      workshopStats: "워크샵 통계",
      totalCandidates: "총 후보 {{count}}개",
      uniqueMods: "고유 항목 {{count}}개",
      duplicates: "중복 {{count}}개",
      skippedSymlinks: "건너뛴 심볼릭 링크 {{count}}개",
      noWorkshopScan: "워크샵 스캔 결과가 없습니다.",
    },
  },

  mods: {
    title: "설치된 모드",
    description:
      "실제 스캔 결과를 기반으로 워크샵 콘텐츠를 표시합니다. 게임 필터를 활용하여 특정 타이틀의 모드만 빠르게 확인할 수 있습니다.",
    searchPlaceholder: "모드 검색",
    allGames: "모든 게임",
    scanLibrary: "라이브러리 스캔",
    scanningLibrary: "스캔 중...",
    columns: {
      modName: "모드 이름",
      game: "게임",
      languages: "지원 언어",
      libraryPath: "라이브러리 경로",
      warnings: "경고 / 참고",
      actions: "작업 제어",
    },
    status: {
      healthy: "정상",
      pathIssue: "경로 확인 필요",
    },
    lastUpdated: "마지막 업데이트",
    workshopId: "워크샵 ID",
    workshopRoot: "워크샵 루트",
    noWarnings: "경고 없음",
    job: {
      schedulable: "예약 가능",
      pending: "준비 중",
      running: "진행 중",
      queued: "대기 중",
      cancelRequested: "중단 요청됨",
      progress: "진행률 {{percent}}%",
      waitingStart: "시작 대기 중",
      queuePosition: "대기열 {{position}}번",
      noQueueInfo: "대기열 정보 없음",
      notInQueue: "대기열에 없음",
    },
    empty: {
      title: "표시할 모드가 없습니다.",
      filterTitle: "{{game}}에 해당하는 모드를 찾지 못했습니다.",
      searchTitle: "검색 조건과 일치하는 모드를 찾지 못했습니다.",
      description:
        "Steam을 실행하여 워크샵 콘텐츠를 다운로드한 뒤, 상단의 스캔 버튼을 눌러 목록을 새로고침하세요.",
      filterDesc:
        "다른 게임을 선택하거나 라이브러리 스캔을 다시 실행해 보세요.",
      searchDesc:
        "다른 키워드로 검색하거나 검색어를 지워 전체 목록을 확인해 보세요.",
    },
    nextSteps: {
      title: "다음 단계",
      items: [
        "진행 상황 탭에서 번역 작업을 예약하고 상태를 모니터링하세요.",
        "Steam이 새 모드를 설치하면 라이브러리 스캔을 다시 실행해 메타데이터를 갱신하세요.",
        "경고가 포함된 모드는 내보내기 전에 검증 도구를 실행해 이상 여부를 확인하세요.",
      ],
    },
    errors: {
      invalidPath:
        "설치 경로를 찾을 수 없어 작업이 실패로 기록되었습니다. 라이브러리 경로를 확인하세요.",
      missingProvider:
        "선택한 번역기의 API 키가 설정되지 않았습니다. 설정 탭에서 API 키를 입력한 뒤 다시 시도해 주세요.",
      missingModel:
        "사용할 모델이 지정되지 않았습니다. 설정 탭에서 제공자별 모델을 선택한 뒤 다시 시도해 주세요.",
      scheduleFailed:
        "작업을 예약하는 중 문제가 발생했습니다. 다시 시도해 주세요.",
      cancelFailed:
        "준비 중인 작업을 취소하지 못했습니다. 잠시 후 다시 시도해 주세요.",
    },
  },

  progress: {
    title: "번역 진행 상황",
    description:
      "현재 활성화된 작업에 대한 진행률과 로그를 표시합니다. 현재 작업을 닫으면 대기열에 있는 다음 작업이 준비됩니다.",
    noJob: {
      title: "진행 중인 작업이 없습니다.",
      description:
        "모드 관리 화면에서 번역할 모드를 선택하면 작업이 대기열에 추가되고 이곳에서 진행 상황을 확인할 수 있습니다.",
      goToMods: "모드 관리로 이동",
    },
    status: {
      pending: "준비 중",
      running: "진행 중",
      completed: "완료됨",
      failed: "실패",
      canceled: "중단됨",
      partial_success: "부분 성공",
      cancelRequested: "중단 요청됨…",
    },
    provider: "번역기",
    model: "모델",
    progress: "진행률 {{percent}}%",
    queueRemaining: "대기열 잔여 {{count}}건",
    language: "언어",
    translated: "{{translated}} / {{total}}개 번역됨",
    selectedFiles: "선택된 파일 {{count}}개",
    outputPath: "출력 경로",
    openFolder: "폴더 열기",
    dismissJob: "작업 닫기",
    cancel: "중단",
    cancelPending: "취소",
    cancelRequested: "중단 요청됨…",
    files: {
      title: "번역 대상 파일",
      description:
        "자동으로 감지된 언어 파일이 선택됩니다. 필요에 따라 번역할 파일을 조정하세요.",
      selected: "선택된 파일 {{count}}개",
      total: "총 {{count}}개",
      autoRecommended: "자동 추천 {{count}}개",
      selectAll: "전체 선택",
      deselectAll: "전체 선택 해제",
      loading: "파일을 불러오는 중입니다.",
      noFiles: "표시할 텍스트 파일이 없습니다.",
      noLanguageFiles:
        "알려진 언어 파일을 찾지 못했습니다. 번역할 파일을 수동으로 선택해 주세요.",
      auto: "자동",
    },
    targetLanguage: "목표 언어",
    outputFolder: "출력 폴더",
    outputPlaceholder: "비워두면 원본 파일 옆에 저장",
    startTranslation: "번역 시작",
    translating: "번역 진행 중",
    preparing: "준비 중...",
    selectFilesError: "번역할 파일을 하나 이상 선택해 주세요.",
    apiKeyWarning:
      '{{provider}} API 키가 "{{status}}" 상태입니다. 설정에서 키를 다시 확인하지 않으면 번역이 실패할 수 있습니다.',
    logs: {
      title: "실시간 로그",
      noLogs: "아직 표시할 로그가 없습니다.",
    },
    history: {
      title: "완료된 작업 기록",
      recent: "최근 {{count}}건",
      noHistory: "아직 완료된 작업 기록이 없습니다.",
      completedAt: "완료",
      model: "모델",
      language: "언어",
      failedFiles: "실패한 파일",
      outputPath: "출력 경로",
    },
    retry: {
      label: "{{seconds}}초 후 재시도 ({{attempt}}/{{max}})",
      retryNow: "지금 재시도",
      retrying: "재시도 중…",
    },
    resume: {
      fromLine: "{{line}}번 줄부터 재개",
      fromLastLine: "마지막 줄부터 재개",
      restart: "파일 처음부터 재시작",
      resuming: "재개 중…",
      restarting: "재시작 중…",
    },
    errors: {
      filesWithErrors: "오류가 발생한 파일",
      code: "코드",
      cancelFailed:
        "작업 중단 요청에 실패했습니다. 잠시 후 다시 시도해 주세요.",
      openFolderFailed: "출력 경로 정보를 찾을 수 없습니다.",
      desktopOnly: "출력 폴더 열기는 데스크톱 환경에서만 지원됩니다.",
    },
  },

  settings: {
    title: "작업 공간 설정",
    description:
      "번역 엔진, Steam 연동, 그리고 처리량 제한을 조정할 수 있습니다.",
    providers: {
      title: "AI 제공자",
      description: "번역 작업에 사용할 모델을 선택하세요.",
      gemini: {
        name: "제미니",
        description: "Google 기반 컨텍스트 확장 모델을 사용합니다.",
      },
      gpt: {
        name: "GPT",
        description: "긴 컨텍스트와 안정적인 번역 품질을 제공합니다.",
      },
      claude: {
        name: "클로드",
        description: "Anthropic의 분석 중심 모델로 세밀한 표현에 강합니다.",
      },
      grok: {
        name: "그록",
        description: "xAI 모델을 통해 빠른 응답과 유연한 문체를 제공합니다.",
      },
    },
    apiKeys: {
      title: "API 키 설정",
      description:
        "각 제공자별 API 키를 직접 입력해 Rust 백엔드와의 연동을 준비하세요. 빈 값으로 저장하면 키가 제거됩니다.",
      missingKeyWarning:
        "{{provider}} API 키가 설정되지 않았습니다. 번역 작업을 예약하기 전에 키를 입력하세요.",
      stored: "저장된 키: {{masked}}",
      noKey: "저장된 키가 없습니다.",
      securityNote:
        "API 키는 로컬 장치에 암호화되지 않은 상태로 저장되므로 보안에 유의하세요.",
      status: {
        checking: "확인 중…",
        valid: "키 정상",
        unauthorized: "인증 실패",
        forbidden: "권한 없음",
        networkError: "네트워크 오류",
        unknown: "미확인",
      },
      validation: {
        checking:
          "{{provider}} API 키 상태를 확인하는 중입니다. 잠시만 기다려 주세요.",
        notChecked:
          '{{provider}} API 키가 아직 확인되지 않았습니다. 키를 저장하거나 "키 확인" 버튼을 눌러 상태를 확인해 주세요.',
        valid: "{{provider}} API 키가 정상입니다.",
        validWithModels:
          "{{provider}} API 키가 정상입니다. 사용 가능한 모델: {{models}}.",
        validNoModels:
          "{{provider}} API 키가 정상입니다. 확인된 모델이 아직 없습니다. 다른 모델을 선택해 검증해 주세요.",
        unauthorized:
          "{{provider}} API 키가 401 Unauthorized 응답으로 거부되었습니다. 키를 다시 확인해 주세요.",
        forbidden:
          "{{provider}} 키는 인식되었지만 선택한 모델이 허용되지 않았습니다. 다른 모델을 선택하거나 플랜을 확인해 주세요.",
        networkError:
          "{{provider}} 제공자에 연결하지 못했습니다. 네트워크 상태를 확인한 뒤 다시 시도해 주세요.",
      },
      model: {
        label: "모델",
        placeholder: "사용 가능한 모델이 없습니다",
        liveBadge: "실시간",
        networkErrorBadge: "네트워크 오류",
        fallbackBadge: "대체 목록",
        verifiedGroup: "이 키로 확인된 모델",
        otherGroup: "기타 알려진 모델",
        knownGroup: "알려진 모델",
        checkingHint: "키 상태를 확인하는 중입니다. 잠시만 기다려 주세요.",
        networkErrorHint:
          "네트워크 오류로 기본 모델 목록을 사용합니다. 실행 시 실패할 수 있습니다.",
        noOptionsHint:
          "표시할 모델이 없습니다. 키를 확인하거나 다른 모델 ID를 수동으로 검증해 주세요.",
        verifiedHint:
          "이 키로 확인된 모델이 먼저 표시됩니다. 아래의 기타 모델은 추가 검증이 필요할 수 있습니다.",
        fallbackHint:
          "아직 검증된 모델이 없어 알려진 기본 모델 목록을 표시합니다. 사용 전 키를 확인해 주세요.",
      },
      verify: "키 확인 / 모델 새로고침",
      verifying: "확인 중…",
    },
    steam: {
      title: "Steam 연동",
      pathLabel: "Steam 경로",
      placeholder: "예: C:/Program Files (x86)/Steam",
      detect: "자동 감지",
      scan: "스캔",
      scanning: "스캔 중...",
      noteDetected: "자동으로 감지된 경로: {path}",
      noteNotFound: "자동 감지에 실패했습니다. 경로를 직접 입력해 주세요.",
      noteEmpty: "경로를 입력해 주세요.",
      noteDone: "스캔이 완료되었습니다.",
      noteError: "스캔 중 오류가 발생했습니다.",
      scanNotes: "최근 스캔 메모",
    },
    limits: {
      title: "처리량 제한",
      description: "번역 큐와 토큰 버킷을 조정해 공급자 제한을 준수하세요.",
      concurrency: "동시 요청 수",
      workers: "워커 수",
      bucket: "버킷 크기",
      refillMs: "리필 주기 (ms)",
      autoTune: "429 응답 시 동시성 자동 조정",
      hints: {
        concurrency: "동시에 처리할 번역 요청 수입니다.",
        workers: "백그라운드에서 동작하는 워커 스레드 수입니다.",
        bucket: "토큰 버킷의 최대 토큰 수입니다.",
        refillMs: "토큰이 리필되는 주기(밀리초)입니다.",
        autoTune: "429 응답을 받으면 동시성을 자동으로 낮춥니다.",
      },
    },
    retry: {
      title: "재시도 정책",
      description:
        "제공자별 최대 시도 횟수와 지연 시간을 조정하고, 재시도 대상 오류를 선택할 수 있습니다.",
      maxRetries: {
        label: "최대 재시도 횟수",
        hint: "초기 요청 이후 재시도할 최대 횟수입니다.",
      },
      initialDelayMs: {
        label: "초기 지연 (ms)",
        hint: "첫 번째 재시도 전 대기 시간입니다.",
      },
      multiplier: {
        label: "지연 배율",
        hint: "재시도마다 지연 시간에 곱해지는 값입니다.",
      },
      maxDelayMs: {
        label: "최대 지연 (ms)",
        hint: "재시도 간 대기 시간의 상한입니다.",
      },
      respectServerHints: {
        label: "서버 Retry-After 헤더 준수",
        hint: "서버가 제공하는 재시도 지연 시간을 우선 적용합니다.",
      },
      autoTune429: {
        label: "429 응답 시 동시성 자동 조정",
        hint: "Rate limit 응답 시 동시 요청 수를 자동으로 줄입니다.",
      },
      maxAttempts: "최대 시도 횟수",
      initialDelay: "초기 지연 (ms)",
      maxDelay: "최대 지연 (ms)",
      retryableErrors: "재시도 대상 오류",
      errorCodes: {
        RATE_LIMITED: "429: 요청 제한 (Rate Limit)",
        NETWORK_TRANSIENT: "네트워크/연결 오류",
        SERVER_TRANSIENT: "서버 오류 (5xx)",
      },
    },
    rules: {
      title: "번역 규칙 및 로깅",
      backendLogging: "백엔드 상세 로그 남기기",
      serverHints: "서버 재시도 힌트 우선 사용",
      placeholderGuard: "플레이스홀더 일치 검증 강제",
      validationMode: "검증 모드",
      validationModes: {
        strict: "엄격 (Strict)",
        relaxed_xml: "느슨 (Relaxed XML)",
      },
      validationModeDesc:
        "느슨 모드: 수식/라텍스 무시, XML 태그 경계 내 텍스트만 검증, 자동 복구 활성화 (권장)",
      dllResources: "DLL 리소스 우선 처리 (Mono.Cecil)",
      qualitySampling: "품질 샘플링(5%) 수행",
    },
  },

  languages: {
    en: "영어",
    ko: "한국어",
    ja: "일본어",
    zh: "중국어",
    "zh-cn": "중국어 (간체)",
    "zh-tw": "중국어 (번체)",
    ru: "러시아어",
    es: "스페인어",
    fr: "프랑스어",
    de: "독일어",
    pt: "포르투갈어",
    pl: "폴란드어",
    it: "이탈리아어",
  },
};
