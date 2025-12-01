import type { TranslationDictionary } from "./types";

export const en: TranslationDictionary = {
  common: {
    save: "Save",
    cancel: "Cancel",
    enabled: "Enabled",
    disabled: "Disabled",
    run: "Run",
    preparing: "Preparing...",
    loading: "Loading...",
    error: "Error",
    success: "Success",
    warning: "Warning",
    info: "Info",
    confirm: "Confirm",
    close: "Close",
    delete: "Delete",
    edit: "Edit",
    add: "Add",
    search: "Search",
    filter: "Filter",
    refresh: "Refresh",
    back: "Back",
    next: "Next",
    previous: "Previous",
    done: "Done",
    retry: "Retry",
    noData: "No data available.",
    language: "Language",
  },

  navigation: {
    dashboard: "Dashboard",
    dashboardDesc: "Overview of recent status and progress",
    mods: "Mod Management",
    modsDesc: "Browse installed workshop content",
    progress: "Progress",
    progressDesc: "Monitor translation pipeline",
    settings: "Settings",
    settingsDesc: "Configure translator and scan options",
  },

  app: {
    title: "Mod Translator",
    controlCenter: "Control Center",
    description:
      "Scan your Steam library, run translation jobs, and monitor your projects at a glance.",
    workspace: "Mod Translator Workspace",
    workspaceDesc:
      "Connect to Steam, orchestrate AI translators, and monitor quality.",
    policyAcknowledged: "Policy acknowledgment recorded.",
    policyRequired: "Please review and agree to the policy.",
  },

  dashboard: {
    title: "Today's Summary",
    description:
      "View policy consent status, library scan results, and workshop warnings in one place.",
    summaryTitle: "Today's Summary",
    summaryDesc:
      "View policy consent status, library scan results, and workshop warnings in one place.",
    detectedLibraries: "Detected Libraries",
    scanning: "Scanning",
    healthyPaths: "{{count}} healthy paths",
    foundMods: "Found Mods",
    workshopRoots: "{{count}} workshop roots",
    noWorkshopRoots: "No workshop roots found",
    warnings: "Warnings",
    noWarnings: "No warnings requiring action.",
    reviewWarnings: "Review warnings and take necessary actions.",
    highlights: {
      libraryScan: "Library Scan Results",
      workshopPath: "Workshop Path",
      jobStatus: "Translation Job Status",
    },
    notes: {
      scanningWorkshop: "Scanning for workshop content.",
      waitingForSteam:
        "Scanning will start automatically once Steam path is confirmed.",
      librariesDetected: "Libraries detected successfully.",
      workshopConnected: "Found libraries with connected workshop content.",
      workshopNotFound:
        "Workshop path not found. Try running Steam once and retry.",
    },
    job: {
      noJobs: "No scheduled translation jobs.",
      addJobHint: "Add a new job from the Mod Management tab.",
      recentCompleted: "{{count}} recently completed",
      failed: "{{count}} failed",
      running: "{{count}} running",
      pending: "{{count}} pending",
      queued: "{{count}} queued",
      completed: "{{count}} completed",
    },
    quickActions: {
      rescan: "Rescan Library",
      rescanDesc:
        "Re-read libraryfolders.vdf from Steam path to refresh mod list.",
      viewJob: "View {{modName}} Job",
      scheduleJob: "Schedule Translation Job",
      checkQueue: "Check Queued Jobs",
      qualityGuard: "Quality Guard Settings",
      qualityGuardDesc:
        "Translation quality verification tools will be integrated in future updates.",
    },
    gameSummary: {
      title: "Mods by Game",
      noData: "No scanned mods to display distribution.",
      modsDetected: "{{count}} mods detected",
      warnings: "{{count}} warnings",
    },
    pipeline: {
      title: "Pipeline Snapshot",
      stages: [
        "Extract workshop archives",
        "Identify file formats and classify text assets",
        "Parse JSON/INI/XML/RESX resources",
        "Lock placeholders and run translation",
        "Validate placeholders and markup",
        "Repackage resources or generate patches",
      ],
    },
    debug: {
      title: "Debug: Library Detection Pipeline",
      description:
        "Quickly inspect canonicalized paths and duplicate/symlink skip information.",
      candidates: "Detected Path Candidates",
      finalLibraries: "Final Library Set",
      noCandidates: "No path candidates found.",
      noFinalLibraries: "No final libraries found.",
      original: "Original",
      canonical: "Canonical Path",
      key: "Key",
      status: "Status",
      note: "Note",
      noCanonicalPaths: "No canonicalized paths.",
      rejectedCandidates: "Rejected Candidates",
      workshopStats: "Workshop Statistics",
      totalCandidates: "{{count}} total candidates",
      uniqueMods: "{{count}} unique mods",
      duplicates: "{{count}} duplicates",
      skippedSymlinks: "{{count}} skipped symlinks",
      noWorkshopScan: "No workshop scan results.",
    },
  },

  mods: {
    title: "Installed Mods",
    description:
      "Display workshop content based on actual scan results. Use the game filter to quickly view mods for specific titles.",
    searchPlaceholder: "Search mods",
    allGames: "All Games",
    scanLibrary: "Scan Library",
    scanningLibrary: "Scanning...",
    columns: {
      modName: "Mod Name",
      game: "Game",
      languages: "Languages",
      libraryPath: "Library Path",
      warnings: "Warnings / Notes",
      actions: "Actions",
    },
    status: {
      healthy: "Healthy",
      pathIssue: "Path issue",
    },
    lastUpdated: "Last updated",
    workshopId: "Workshop ID",
    workshopRoot: "Workshop root",
    noWarnings: "No warnings",
    job: {
      schedulable: "Schedulable",
      pending: "Pending",
      running: "Running",
      queued: "Queued",
      cancelRequested: "Cancel requested",
      progress: "{{percent}}% progress",
      waitingStart: "Waiting to start",
      queuePosition: "Queue position {{position}}",
      noQueueInfo: "No queue info",
      notInQueue: "Not in queue",
    },
    empty: {
      title: "No mods to display.",
      filterTitle: "No mods found for {{game}}.",
      searchTitle: "No mods matching search criteria.",
      description:
        "Run Steam to download workshop content, then click the scan button above to refresh the list.",
      filterDesc: "Try selecting a different game or rescan the library.",
      searchDesc:
        "Try different keywords or clear the search to view all mods.",
    },
    nextSteps: {
      title: "Next Steps",
      items: [
        "Schedule translation jobs and monitor status from the Progress tab.",
        "Rescan the library when Steam installs new mods to update metadata.",
        "Run validation tools on mods with warnings before exporting.",
      ],
    },
    errors: {
      invalidPath:
        "Installation path not found. Job marked as failed. Check library path.",
      missingProvider:
        "API key for selected translator not configured. Enter API key in Settings and try again.",
      missingModel:
        "No model specified. Select a model for the provider in Settings and try again.",
      scheduleFailed: "Failed to schedule job. Please try again.",
      cancelFailed: "Failed to cancel pending job. Please try again later.",
    },
  },

  progress: {
    title: "Translation Progress",
    description:
      "Shows progress and logs for the currently active job. Dismissing the current job will prepare the next queued job.",
    noJob: {
      title: "No active job.",
      description:
        "Select a mod from the Mod Management screen to add it to the queue and view progress here.",
      goToMods: "Go to Mod Management",
    },
    status: {
      pending: "Pending",
      running: "Running",
      completed: "Completed",
      failed: "Failed",
      canceled: "Canceled",
      partial_success: "Partial Success",
      cancelRequested: "Cancel requested...",
    },
    provider: "Provider",
    model: "Model",
    progress: "{{percent}}% progress",
    queueRemaining: "{{count}} jobs remaining in queue",
    language: "Language",
    translated: "{{translated}} / {{total}} translated",
    selectedFiles: "{{count}} files selected",
    outputPath: "Output path",
    openFolder: "Open Folder",
    dismissJob: "Dismiss Job",
    cancel: "Cancel",
    cancelPending: "Cancel",
    cancelRequested: "Cancel requested...",
    files: {
      title: "Files to Translate",
      description:
        "Automatically detected language files are selected. Adjust files to translate as needed.",
      selected: "{{count}} selected",
      total: "{{count}} total",
      autoRecommended: "{{count}} auto-recommended",
      selectAll: "Select All",
      deselectAll: "Deselect All",
      loading: "Loading files...",
      noFiles: "No text files to display.",
      noLanguageFiles:
        "No known language files found. Please manually select files to translate.",
      auto: "Auto",
    },
    targetLanguage: "Target Language",
    outputFolder: "Output Folder",
    outputPlaceholder: "Leave empty to save next to original files",
    startTranslation: "Start Translation",
    translating: "Translating...",
    preparing: "Preparing...",
    selectFilesError: "Please select at least one file to translate.",
    apiKeyWarning:
      '{{provider}} API key is "{{status}}". Translation may fail if key is not verified in Settings.',
    logs: {
      title: "Live Logs",
      noLogs: "No logs to display yet.",
    },
    history: {
      title: "Completed Job History",
      recent: "{{count}} recent",
      noHistory: "No completed job history yet.",
      completedAt: "Completed",
      model: "Model",
      language: "Language",
      failedFiles: "Failed Files",
      outputPath: "Output path",
    },
    retry: {
      label: "Retrying in {{seconds}}s ({{attempt}}/{{max}})",
      retryNow: "Retry Now",
      retrying: "Retrying...",
    },
    resume: {
      fromLine: "Resume from line {{line}}",
      fromLastLine: "Resume from last line",
      restart: "Restart from file start",
      resuming: "Resuming...",
      restarting: "Restarting...",
    },
    errors: {
      filesWithErrors: "Files with errors",
      code: "Code",
      cancelFailed: "Failed to request job cancellation. Please try again.",
      openFolderFailed: "Output path information not found.",
      desktopOnly:
        "Opening output folder is only supported in desktop environment.",
    },
  },

  settings: {
    title: "Workspace Settings",
    description:
      "Configure translation engines, Steam integration, and rate limits.",
    providers: {
      title: "AI Providers",
      description: "Select models to use for translation jobs.",
      gemini: {
        name: "Gemini",
        description: "Uses Google-based extended context models.",
      },
      gpt: {
        name: "GPT",
        description: "Provides long context and stable translation quality.",
      },
      claude: {
        name: "Claude",
        description:
          "Anthropic's analysis-focused model excels at nuanced expressions.",
      },
      grok: {
        name: "Grok",
        description:
          "xAI model providing fast responses and flexible writing styles.",
      },
    },
    apiKeys: {
      title: "API Key Settings",
      description:
        "Enter API keys for each provider to prepare Rust backend integration. Saving empty value removes the key.",
      missingKeyWarning:
        "{{provider}} API key is not set. Please enter a key before scheduling translation jobs.",
      stored: "Stored key: {{masked}}",
      noKey: "No key stored.",
      securityNote:
        "API keys are stored unencrypted on local device. Please be mindful of security.",
      status: {
        checking: "Checking...",
        valid: "Key valid",
        unauthorized: "Unauthorized",
        forbidden: "Forbidden",
        networkError: "Network error",
        unknown: "Unknown",
      },
      validation: {
        checking: "Checking {{provider}} API key status. Please wait.",
        notChecked:
          '{{provider}} API key not yet verified. Save key or click "Verify Key" button to check status.',
        valid: "{{provider}} API key is valid.",
        validWithModels:
          "{{provider}} API key is valid. Available models: {{models}}.",
        validNoModels:
          "{{provider}} API key is valid. No models verified yet. Try selecting another model to verify.",
        unauthorized:
          "{{provider}} API key rejected with 401 Unauthorized. Please check the key.",
        forbidden:
          "{{provider}} key recognized but selected model not allowed. Try another model or check your plan.",
        networkError:
          "Could not connect to {{provider}}. Check network status and try again.",
      },
      model: {
        label: "Model",
        placeholder: "No models available",
        liveBadge: "Live",
        networkErrorBadge: "Network Error",
        fallbackBadge: "Fallback List",
        verifiedGroup: "Verified with this key",
        otherGroup: "Other known models",
        knownGroup: "Known models",
        checkingHint: "Checking key status. Please wait.",
        networkErrorHint:
          "Using default model list due to network error. May fail at runtime.",
        noOptionsHint:
          "No models to display. Verify key or manually check another model ID.",
        verifiedHint:
          "Models verified with this key shown first. Other models below may require additional verification.",
        fallbackHint:
          "No verified models yet. Showing known default models. Please verify key before use.",
      },
      verify: "Verify Key / Refresh Models",
      verifying: "Verifying...",
    },
    steam: {
      title: "Steam Integration",
      pathLabel: "Steam Path",
      placeholder: "e.g., C:/Program Files (x86)/Steam",
      detect: "Auto Detect",
      scan: "Scan",
      scanning: "Scanning...",
      noteDetected: "Auto-detected path: {path}",
      noteNotFound: "Auto-detection failed. Please enter path manually.",
      noteEmpty: "Please enter a path.",
      noteDone: "Scan completed.",
      noteError: "Error occurred during scan.",
      scanNotes: "Recent Scan Notes",
    },
    limits: {
      title: "Rate Limits",
      description:
        "Adjust translation queue and token bucket to comply with provider limits.",
      concurrency: "Concurrent Requests",
      workers: "Worker Count",
      bucket: "Bucket Size",
      refillMs: "Refill Interval (ms)",
      autoTune: "Auto-tune concurrency on 429 response",
      hints: {
        concurrency:
          "Number of translation requests to process simultaneously.",
        workers: "Number of background worker threads.",
        bucket: "Maximum tokens in the token bucket.",
        refillMs: "Interval in milliseconds for token refill.",
        autoTune:
          "Automatically lower concurrency when receiving 429 responses.",
      },
    },
    retry: {
      title: "Retry Policy",
      description:
        "Adjust maximum attempts and delay times per provider, and select which errors to retry.",
      maxRetries: {
        label: "Max Retries",
        hint: "Maximum number of retries after initial request.",
      },
      initialDelayMs: {
        label: "Initial Delay (ms)",
        hint: "Wait time before first retry.",
      },
      multiplier: {
        label: "Delay Multiplier",
        hint: "Value multiplied to delay time for each retry.",
      },
      maxDelayMs: {
        label: "Max Delay (ms)",
        hint: "Upper limit for wait time between retries.",
      },
      respectServerHints: {
        label: "Respect server Retry-After header",
        hint: "Prioritize retry delay time provided by server.",
      },
      autoTune429: {
        label: "Auto-tune concurrency on 429 response",
        hint: "Automatically reduce concurrent requests on rate limit responses.",
      },
      maxAttempts: "Max Attempts",
      initialDelay: "Initial Delay (ms)",
      maxDelay: "Max Delay (ms)",
      retryableErrors: "Retryable Errors",
      errorCodes: {
        RATE_LIMITED: "429: Rate Limited",
        NETWORK_TRANSIENT: "Network/Connection Error",
        SERVER_TRANSIENT: "Server Error (5xx)",
      },
    },
    rules: {
      title: "Translation Rules & Logging",
      backendLogging: "Enable backend verbose logging",
      serverHints: "Prefer server retry hints",
      placeholderGuard: "Enforce placeholder match validation",
      validationMode: "Validation Mode",
      validationModes: {
        strict: "Strict",
        relaxed_xml: "Relaxed XML",
      },
      validationModeDesc:
        "Relaxed mode: Ignore math/latex, validate only text within XML tag boundaries, enable auto-recovery (recommended)",
      dllResources: "Prioritize DLL resources (Mono.Cecil)",
      qualitySampling: "Perform quality sampling (5%)",
    },
  },

  languages: {
    en: "English",
    ko: "Korean",
    ja: "Japanese",
    zh: "Chinese",
    "zh-cn": "Chinese (Simplified)",
    "zh-tw": "Chinese (Traditional)",
    ru: "Russian",
    es: "Spanish",
    fr: "French",
    de: "German",
    pt: "Portuguese",
    pl: "Polish",
    it: "Italian",
  },
};
