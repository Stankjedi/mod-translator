/* eslint-disable react-refresh/only-export-components */
import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
  type ReactNode,
} from 'react'

type ToastTone = 'neutral' | 'success' | 'error' | 'warning'

interface ToastRecord {
  id: string
  message: string
  tone: ToastTone
}

interface ToastContextValue {
  showToast: (message: string, tone?: ToastTone, durationMs?: number) => void
}

const DEFAULT_DURATION = 4000

const toneStyles: Record<ToastTone, string> = {
  neutral: 'border-slate-700 bg-slate-900/95 text-slate-100',
  success: 'border-emerald-500/40 bg-emerald-500/15 text-emerald-100',
  error: 'border-rose-500/40 bg-rose-500/15 text-rose-100',
  warning: 'border-amber-500/40 bg-amber-500/15 text-amber-100',
}

const ToastContext = createContext<ToastContextValue | undefined>(undefined)

const createToastId = () =>
  typeof crypto !== 'undefined' && 'randomUUID' in crypto
    ? crypto.randomUUID()
    : `toast-${Date.now()}-${Math.random().toString(16).slice(2)}`

export function ToastProvider({ children }: { children: ReactNode }) {
  const [toasts, setToasts] = useState<ToastRecord[]>([])
  const timersRef = useRef<Record<string, number>>({})

  const dismissToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((toast) => toast.id !== id))

    if (typeof window !== 'undefined') {
      const timeoutId = timersRef.current[id]
      if (timeoutId) {
        window.clearTimeout(timeoutId)
        delete timersRef.current[id]
      }
    }
  }, [])

  const showToast = useCallback<ToastContextValue['showToast']>(
    (message, tone = 'neutral', durationMs = DEFAULT_DURATION) => {
      const trimmed = message.trim()
      if (!trimmed) {
        return
      }

      const id = createToastId()
      setToasts((prev) => [...prev, { id, message: trimmed, tone }])

      if (typeof window !== 'undefined') {
        const timeoutId = window.setTimeout(() => {
          dismissToast(id)
        }, Math.max(1000, durationMs))
        timersRef.current[id] = timeoutId
      }
    },
    [dismissToast],
  )

  const contextValue = useMemo<ToastContextValue>(
    () => ({
      showToast,
    }),
    [showToast],
  )

  return (
    <ToastContext.Provider value={contextValue}>
      {children}
      <div
        className="pointer-events-none fixed inset-x-0 bottom-4 z-[1000] flex justify-center px-4 sm:justify-end sm:px-6"
        aria-live="polite"
        role="status"
      >
        <div className="flex w-full max-w-sm flex-col gap-3">
          {toasts.map((toast) => (
            <div
              key={toast.id}
              className={`pointer-events-auto rounded-xl border px-4 py-3 text-sm shadow-lg shadow-black/40 backdrop-blur ${toneStyles[toast.tone]}`}
            >
              <div className="flex items-start justify-between gap-3">
                <span className="leading-snug">{toast.message}</span>
                <button
                  type="button"
                  onClick={() => dismissToast(toast.id)}
                  className="ml-2 inline-flex h-6 w-6 items-center justify-center rounded-full border border-slate-600/60 text-xs text-slate-300 transition hover:border-slate-400 hover:text-white"
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
  )
}

export function useToast() {
  const context = useContext(ToastContext)
  if (!context) {
    throw new Error('useToast must be used within a ToastProvider')
  }
  return context
}
