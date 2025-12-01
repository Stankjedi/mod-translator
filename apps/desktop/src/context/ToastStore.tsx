/* eslint-disable react-refresh/only-export-components */
import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from "react";

type ToastTone = "neutral" | "success" | "error" | "warning";

// 에러 유형별 정보
export type ErrorType =
  | "network"
  | "api_key_invalid"
  | "rate_limit"
  | "format_error"
  | "file_not_found"
  | "permission_denied"
  | "timeout"
  | "unknown";

interface ErrorInfo {
  type: ErrorType;
  message: string;
  retryAfterMs?: number; // API 한도 초과 시 대기 시간
  suggestion?: string; // 해결 방법 제안
}

// 에러 유형별 아이콘 (SVG path)
const errorIcons: Record<ErrorType, string> = {
  network:
    "M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z",
  api_key_invalid:
    "M18 8h-1V6c0-2.76-2.24-5-5-5S7 3.24 7 6v2H6c-1.1 0-2 .9-2 2v10c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V10c0-1.1-.9-2-2-2zm-6 9c-1.1 0-2-.9-2-2s.9-2 2-2 2 .9 2 2-.9 2-2 2zm3.1-9H8.9V6c0-1.71 1.39-3.1 3.1-3.1 1.71 0 3.1 1.39 3.1 3.1v2z",
  rate_limit:
    "M11.99 2C6.47 2 2 6.48 2 12s4.47 10 9.99 10C17.52 22 22 17.52 22 12S17.52 2 11.99 2zM12 20c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8zm.5-13H11v6l5.25 3.15.75-1.23-4.5-2.67z",
  format_error:
    "M14 2H6c-1.1 0-2 .9-2 2v16c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V8l-6-6zm-1 9h-2V7h2v4zm0 4h-2v-2h2v2z",
  file_not_found:
    "M14 2H6c-1.1 0-2 .9-2 2v16c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V8l-6-6zM6 20V4h7v5h5v11H6z",
  permission_denied:
    "M12 1L3 5v6c0 5.55 3.84 10.74 9 12 5.16-1.26 9-6.45 9-12V5l-9-4zm0 10.99h7c-.53 4.12-3.28 7.79-7 8.94V12H5V6.3l7-3.11v8.8z",
  timeout:
    "M11.99 2C6.47 2 2 6.48 2 12s4.47 10 9.99 10C17.52 22 22 17.52 22 12S17.52 2 11.99 2zM12 20c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8zm1-13h-2v6l5.25 3.15.75-1.23-4.5-2.67z",
  unknown:
    "M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm1 17h-2v-2h2v2zm2.07-7.75l-.9.92C13.45 12.9 13 13.5 13 15h-2v-.5c0-1.1.45-2.1 1.17-2.83l1.24-1.26c.37-.36.59-.86.59-1.41 0-1.1-.9-2-2-2s-2 .9-2 2H8c0-2.21 1.79-4 4-4s4 1.79 4 4c0 .88-.36 1.68-.93 2.25z",
};

// 에러 유형별 기본 해결 제안
const defaultSuggestions: Record<ErrorType, string> = {
  network: "인터넷 연결을 확인하고 다시 시도해주세요.",
  api_key_invalid: "설정에서 API 키가 올바르게 입력되었는지 확인해주세요.",
  rate_limit: "잠시 후 다시 시도해주세요.",
  format_error: "파일 형식이 올바른지 확인해주세요.",
  file_not_found: "파일이 존재하는지 확인해주세요.",
  permission_denied: "파일 접근 권한을 확인해주세요.",
  timeout: "네트워크 상태를 확인하고 다시 시도해주세요.",
  unknown: "문제가 지속되면 앱을 재시작해주세요.",
};

interface ToastRecord {
  id: string;
  message: string;
  tone: ToastTone;
  errorInfo?: ErrorInfo;
}

interface ToastContextValue {
  showToast: (message: string, tone?: ToastTone, durationMs?: number) => void;
  showError: (error: ErrorInfo, durationMs?: number) => void;
}

const DEFAULT_DURATION = 4000;
const ERROR_DURATION = 6000;

const toneStyles: Record<ToastTone, string> = {
  neutral: "border-slate-700 bg-slate-900/95 text-slate-100",
  success: "border-emerald-500/40 bg-emerald-500/15 text-emerald-100",
  error: "border-rose-500/40 bg-rose-500/15 text-rose-100",
  warning: "border-amber-500/40 bg-amber-500/15 text-amber-100",
};

const ToastContext = createContext<ToastContextValue | undefined>(undefined);

const createToastId = () =>
  typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `toast-${Date.now()}-${Math.random().toString(16).slice(2)}`;

// 에러 메시지에서 에러 유형 감지
export function detectErrorType(message: string): ErrorType {
  const lowerMsg = message.toLowerCase();

  if (
    lowerMsg.includes("network") ||
    lowerMsg.includes("fetch") ||
    lowerMsg.includes("연결")
  ) {
    return "network";
  }
  if (
    lowerMsg.includes("api key") ||
    lowerMsg.includes("unauthorized") ||
    lowerMsg.includes("invalid key") ||
    lowerMsg.includes("api 키")
  ) {
    return "api_key_invalid";
  }
  if (
    lowerMsg.includes("rate limit") ||
    lowerMsg.includes("quota") ||
    lowerMsg.includes("too many") ||
    lowerMsg.includes("429") ||
    lowerMsg.includes("한도")
  ) {
    return "rate_limit";
  }
  if (
    lowerMsg.includes("format") ||
    lowerMsg.includes("parse") ||
    lowerMsg.includes("형식")
  ) {
    return "format_error";
  }
  if (
    lowerMsg.includes("not found") ||
    lowerMsg.includes("존재하지") ||
    lowerMsg.includes("찾을 수 없")
  ) {
    return "file_not_found";
  }
  if (
    lowerMsg.includes("permission") ||
    lowerMsg.includes("access denied") ||
    lowerMsg.includes("권한")
  ) {
    return "permission_denied";
  }
  if (
    lowerMsg.includes("timeout") ||
    lowerMsg.includes("timed out") ||
    lowerMsg.includes("시간 초과")
  ) {
    return "timeout";
  }

  return "unknown";
}

// 남은 시간 포맷
function formatRetryTime(ms: number): string {
  const seconds = Math.ceil(ms / 1000);
  if (seconds < 60) {
    return `${seconds}초`;
  }
  const minutes = Math.ceil(seconds / 60);
  return `${minutes}분`;
}

// 에러 아이콘 컴포넌트
function ErrorIcon({
  type,
  className,
}: {
  type: ErrorType;
  className?: string;
}) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="currentColor"
      className={className || "h-5 w-5 flex-shrink-0"}
    >
      <path d={errorIcons[type]} />
    </svg>
  );
}

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastRecord[]>([]);
  const timersRef = useRef<Record<string, number>>({});

  const dismissToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((toast) => toast.id !== id));

    if (typeof window !== "undefined") {
      const timeoutId = timersRef.current[id];
      if (timeoutId) {
        window.clearTimeout(timeoutId);
        delete timersRef.current[id];
      }
    }
  }, []);

  const showToast = useCallback<ToastContextValue["showToast"]>(
    (message, tone = "neutral", durationMs = DEFAULT_DURATION) => {
      const trimmed = message.trim();
      if (!trimmed) {
        return;
      }

      const id = createToastId();
      setToasts((prev) => [...prev, { id, message: trimmed, tone }]);

      if (typeof window !== "undefined") {
        const timeoutId = window.setTimeout(
          () => {
            dismissToast(id);
          },
          Math.max(1000, durationMs),
        );
        timersRef.current[id] = timeoutId;
      }
    },
    [dismissToast],
  );

  const showError = useCallback<ToastContextValue["showError"]>(
    (errorInfo, durationMs = ERROR_DURATION) => {
      const id = createToastId();

      // rate_limit일 경우 대기 시간만큼 더 길게 표시
      const duration = errorInfo.retryAfterMs
        ? Math.max(durationMs, errorInfo.retryAfterMs + 2000)
        : durationMs;

      setToasts((prev) => [
        ...prev,
        {
          id,
          message: errorInfo.message,
          tone: "error",
          errorInfo,
        },
      ]);

      if (typeof window !== "undefined") {
        const timeoutId = window.setTimeout(
          () => {
            dismissToast(id);
          },
          Math.max(1000, duration),
        );
        timersRef.current[id] = timeoutId;
      }
    },
    [dismissToast],
  );

  const contextValue = useMemo<ToastContextValue>(
    () => ({
      showToast,
      showError,
    }),
    [showToast, showError],
  );

  return (
    <ToastContext.Provider value={contextValue}>
      {children}
      <div
        className="pointer-events-none fixed inset-x-0 bottom-4 z-[1000] flex justify-center px-4 sm:justify-end sm:px-6"
        aria-live="polite"
        role="status"
      >
        <div className="flex w-full max-w-md flex-col gap-3">
          {toasts.map((toast) => (
            <div
              key={toast.id}
              className={`pointer-events-auto rounded-xl border px-4 py-3 text-sm shadow-lg shadow-black/40 backdrop-blur ${toneStyles[toast.tone]}`}
            >
              <div className="flex items-start gap-3">
                {/* 에러 아이콘 */}
                {toast.errorInfo && (
                  <ErrorIcon
                    type={toast.errorInfo.type}
                    className="mt-0.5 h-5 w-5 flex-shrink-0 opacity-80"
                  />
                )}

                <div className="flex-1 min-w-0">
                  {/* 메시지 */}
                  <span className="block leading-snug">{toast.message}</span>

                  {/* 대기 시간 표시 (rate_limit) */}
                  {toast.errorInfo?.retryAfterMs && (
                    <span className="mt-1 block text-xs opacity-75">
                      ⏱️ {formatRetryTime(toast.errorInfo.retryAfterMs)} 후
                      재시도 가능
                    </span>
                  )}

                  {/* 해결 제안 */}
                  {toast.errorInfo && (
                    <span className="mt-1 block text-xs opacity-60">
                      💡{" "}
                      {toast.errorInfo.suggestion ||
                        defaultSuggestions[toast.errorInfo.type]}
                    </span>
                  )}
                </div>

                {/* 닫기 버튼 */}
                <button
                  type="button"
                  onClick={() => dismissToast(toast.id)}
                  className="ml-2 inline-flex h-6 w-6 flex-shrink-0 items-center justify-center rounded-full border border-slate-600/60 text-xs text-slate-300 transition hover:border-slate-400 hover:text-white"
                  aria-label="닫기"
                >
                  ×
                </button>
              </div>
            </div>
          ))}
        </div>
      </div>
    </ToastContext.Provider>
  );
}

export function useToast() {
  const context = useContext(ToastContext);
  if (!context) {
    throw new Error("useToast must be used within a ToastProvider");
  }
  return context;
}
