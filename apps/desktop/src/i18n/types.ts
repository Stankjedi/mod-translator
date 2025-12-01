/**
 * i18n 타입 정의
 */

export type SupportedLocale = "ko" | "en";

export interface TranslationDictionary {
  common: {
    save: string;
    cancel: string;
    enabled: string;
    disabled: string;
    run: string;
    preparing: string;
    loading: string;
    error: string;
    success: string;
    warning: string;
    info: string;
    confirm: string;
    close: string;
    delete: string;
    edit: string;
    add: string;
    search: string;
    filter: string;
    refresh: string;
    back: string;
    next: string;
    previous: string;
    done: string;
    retry: string;
    noData: string;
    language: string;
  };

  navigation: {
    dashboard: string;
    dashboardDesc: string;
    mods: string;
    modsDesc: string;
    progress: string;
    progressDesc: string;
    settings: string;
    settingsDesc: string;
  };

  app: {
    title: string;
    controlCenter: string;
    description: string;
    workspace: string;
    workspaceDesc: string;
    policyAcknowledged: string;
    policyRequired: string;
  };

  dashboard: {
    title: string;
    description: string;
    summaryTitle: string;
    summaryDesc: string;
    detectedLibraries: string;
    scanning: string;
    healthyPaths: string;
    foundMods: string;
    workshopRoots: string;
    noWorkshopRoots: string;
    warnings: string;
    noWarnings: string;
    reviewWarnings: string;
    highlights: {
      libraryScan: string;
      workshopPath: string;
      jobStatus: string;
    };
    notes: {
      scanningWorkshop: string;
      waitingForSteam: string;
      librariesDetected: string;
      workshopConnected: string;
      workshopNotFound: string;
    };
    job: {
      noJobs: string;
      addJobHint: string;
      recentCompleted: string;
      failed: string;
      running: string;
      pending: string;
      queued: string;
      completed: string;
    };
    quickActions: {
      rescan: string;
      rescanDesc: string;
      viewJob: string;
      scheduleJob: string;
      checkQueue: string;
      qualityGuard: string;
      qualityGuardDesc: string;
    };
    gameSummary: {
      title: string;
      noData: string;
      modsDetected: string;
      warnings: string;
    };
    pipeline: {
      title: string;
      stages: string[];
    };
    debug: {
      title: string;
      description: string;
      candidates: string;
      finalLibraries: string;
      noCandidates: string;
      noFinalLibraries: string;
      original: string;
      canonical: string;
      key: string;
      status: string;
      note: string;
      noCanonicalPaths: string;
      rejectedCandidates: string;
      workshopStats: string;
      totalCandidates: string;
      uniqueMods: string;
      duplicates: string;
      skippedSymlinks: string;
      noWorkshopScan: string;
    };
  };

  mods: {
    title: string;
    description: string;
    searchPlaceholder: string;
    allGames: string;
    scanLibrary: string;
    scanningLibrary: string;
    columns: {
      modName: string;
      game: string;
      languages: string;
      libraryPath: string;
      warnings: string;
      actions: string;
    };
    status: {
      healthy: string;
      pathIssue: string;
    };
    lastUpdated: string;
    workshopId: string;
    workshopRoot: string;
    noWarnings: string;
    job: {
      schedulable: string;
      pending: string;
      running: string;
      queued: string;
      cancelRequested: string;
      progress: string;
      waitingStart: string;
      queuePosition: string;
      noQueueInfo: string;
      notInQueue: string;
    };
    empty: {
      title: string;
      filterTitle: string;
      searchTitle: string;
      description: string;
      filterDesc: string;
      searchDesc: string;
    };
    nextSteps: {
      title: string;
      items: string[];
    };
    errors: {
      invalidPath: string;
      missingProvider: string;
      missingModel: string;
      scheduleFailed: string;
      cancelFailed: string;
    };
  };

  progress: {
    title: string;
    description: string;
    noJob: {
      title: string;
      description: string;
      goToMods: string;
    };
    status: {
      pending: string;
      running: string;
      completed: string;
      failed: string;
      canceled: string;
      partial_success: string;
      cancelRequested: string;
    };
    provider: string;
    model: string;
    progress: string;
    queueRemaining: string;
    language: string;
    translated: string;
    selectedFiles: string;
    outputPath: string;
    openFolder: string;
    dismissJob: string;
    cancel: string;
    cancelPending: string;
    cancelRequested: string;
    files: {
      title: string;
      description: string;
      selected: string;
      total: string;
      autoRecommended: string;
      selectAll: string;
      deselectAll: string;
      loading: string;
      noFiles: string;
      noLanguageFiles: string;
      auto: string;
    };
    targetLanguage: string;
    outputFolder: string;
    outputPlaceholder: string;
    startTranslation: string;
    translating: string;
    preparing: string;
    selectFilesError: string;
    apiKeyWarning: string;
    logs: {
      title: string;
      noLogs: string;
    };
    history: {
      title: string;
      recent: string;
      noHistory: string;
      completedAt: string;
      model: string;
      language: string;
      failedFiles: string;
      outputPath: string;
    };
    retry: {
      label: string;
      retryNow: string;
      retrying: string;
    };
    resume: {
      fromLine: string;
      fromLastLine: string;
      restart: string;
      resuming: string;
      restarting: string;
    };
    errors: {
      filesWithErrors: string;
      code: string;
      cancelFailed: string;
      openFolderFailed: string;
      desktopOnly: string;
    };
  };

  settings: {
    title: string;
    description: string;
    providers: {
      title: string;
      description: string;
      gemini: {
        name: string;
        description: string;
      };
      gpt: {
        name: string;
        description: string;
      };
      claude: {
        name: string;
        description: string;
      };
      grok: {
        name: string;
        description: string;
      };
    };
    apiKeys: {
      title: string;
      description: string;
      missingKeyWarning: string;
      stored: string;
      noKey: string;
      securityNote: string;
      status: {
        checking: string;
        valid: string;
        unauthorized: string;
        forbidden: string;
        networkError: string;
        unknown: string;
      };
      validation: {
        checking: string;
        notChecked: string;
        valid: string;
        validWithModels: string;
        validNoModels: string;
        unauthorized: string;
        forbidden: string;
        networkError: string;
      };
      model: {
        label: string;
        placeholder: string;
        liveBadge: string;
        networkErrorBadge: string;
        fallbackBadge: string;
        verifiedGroup: string;
        otherGroup: string;
        knownGroup: string;
        checkingHint: string;
        networkErrorHint: string;
        noOptionsHint: string;
        verifiedHint: string;
        fallbackHint: string;
      };
      verify: string;
      verifying: string;
    };
    steam: {
      title: string;
      pathLabel: string;
      placeholder: string;
      detect: string;
      scan: string;
      scanning: string;
      noteDetected: string;
      noteNotFound: string;
      noteEmpty: string;
      noteDone: string;
      noteError: string;
      scanNotes: string;
    };
    limits: {
      title: string;
      description: string;
      concurrency: string;
      workers: string;
      bucket: string;
      refillMs: string;
      autoTune: string;
      hints: {
        concurrency: string;
        workers: string;
        bucket: string;
        refillMs: string;
        autoTune: string;
      };
    };
    retry: {
      title: string;
      description: string;
      maxRetries: {
        label: string;
        hint: string;
      };
      initialDelayMs: {
        label: string;
        hint: string;
      };
      multiplier: {
        label: string;
        hint: string;
      };
      maxDelayMs: {
        label: string;
        hint: string;
      };
      respectServerHints: {
        label: string;
        hint: string;
      };
      autoTune429: {
        label: string;
        hint: string;
      };
      maxAttempts: string;
      initialDelay: string;
      maxDelay: string;
      retryableErrors: string;
      errorCodes: {
        RATE_LIMITED: string;
        NETWORK_TRANSIENT: string;
        SERVER_TRANSIENT: string;
      };
    };
    rules: {
      title: string;
      backendLogging: string;
      serverHints: string;
      placeholderGuard: string;
      validationMode: string;
      validationModes: {
        strict: string;
        relaxed_xml: string;
      };
      validationModeDesc: string;
      dllResources: string;
      qualitySampling: string;
    };
  };

  languages: {
    en: string;
    ko: string;
    ja: string;
    zh: string;
    "zh-cn": string;
    "zh-tw": string;
    ru: string;
    es: string;
    fr: string;
    de: string;
    pt: string;
    pl: string;
    it: string;
  };
}
