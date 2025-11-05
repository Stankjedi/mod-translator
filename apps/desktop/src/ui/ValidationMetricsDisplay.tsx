import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { ValidationMetrics } from '../types/core'
import Chip from './Chip'

interface ValidationMetricsDisplayProps {
  autoRefresh?: boolean
  refreshInterval?: number
}

export function ValidationMetricsDisplay({
  autoRefresh = false,
  refreshInterval = 5000,
}: ValidationMetricsDisplayProps) {
  const [metrics, setMetrics] = useState<ValidationMetrics | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const loadMetrics = async () => {
    setLoading(true)
    setError(null)
    try {
      const result = await invoke<ValidationMetrics>('get_validation_metrics')
      setMetrics(result)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }

  const resetMetrics = async () => {
    try {
      await invoke('reset_validation_metrics')
      await loadMetrics()
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }

  const exportMetrics = async () => {
    try {
      const json = await invoke<string>('export_validation_metrics')
      const blob = new Blob([json], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `validation-metrics-${new Date().toISOString()}.json`
      a.click()
      URL.revokeObjectURL(url)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }

  const downloadLog = async () => {
    try {
      const logPath = await invoke<string>('get_validation_log_file_path')
      alert(`로그 파일 위치: ${logPath}`)
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    }
  }

  useEffect(() => {
    loadMetrics()
  }, [])

  useEffect(() => {
    if (autoRefresh) {
      const interval = setInterval(loadMetrics, refreshInterval)
      return () => clearInterval(interval)
    }
  }, [autoRefresh, refreshInterval])

  if (loading && !metrics) {
    return (
      <div className="text-center py-8 text-slate-400">메트릭 로딩 중...</div>
    )
  }

  if (error) {
    return (
      <div className="border border-rose-500/30 bg-rose-500/5 rounded-lg p-4">
        <div className="text-rose-400">오류: {error}</div>
        <button
          onClick={loadMetrics}
          className="mt-2 px-3 py-1.5 text-xs font-medium text-white bg-rose-500 hover:bg-rose-600 rounded transition-colors"
        >
          다시 시도
        </button>
      </div>
    )
  }

  if (!metrics) {
    return null
  }

  const failureRate = metrics.totalValidations > 0
    ? ((metrics.totalFailures / metrics.totalValidations) * 100).toFixed(1)
    : '0.0'

  const autofixRate = metrics.autofixAttempts > 0
    ? ((metrics.autofixSuccesses / metrics.autofixAttempts) * 100).toFixed(1)
    : '0.0'

  const retryRate = metrics.retryAttempts > 0
    ? ((metrics.retrySuccesses / metrics.retryAttempts) * 100).toFixed(1)
    : '0.0'

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-slate-200">검증 메트릭</h2>
        <div className="flex gap-2">
          <button
            onClick={loadMetrics}
            disabled={loading}
            className="px-3 py-1.5 text-xs font-medium text-slate-300 bg-slate-800 border border-slate-700 rounded hover:bg-slate-700 transition-colors disabled:opacity-50"
          >
            {loading ? '로딩 중...' : '새로고침'}
          </button>
          <button
            onClick={exportMetrics}
            className="px-3 py-1.5 text-xs font-medium text-slate-300 bg-slate-800 border border-slate-700 rounded hover:bg-slate-700 transition-colors"
          >
            내보내기
          </button>
          <button
            onClick={downloadLog}
            className="px-3 py-1.5 text-xs font-medium text-slate-300 bg-slate-800 border border-slate-700 rounded hover:bg-slate-700 transition-colors"
          >
            로그 위치
          </button>
          <button
            onClick={resetMetrics}
            className="px-3 py-1.5 text-xs font-medium text-rose-300 bg-rose-500/10 border border-rose-500/30 rounded hover:bg-rose-500/20 transition-colors"
          >
            초기화
          </button>
        </div>
      </div>

      {/* Summary cards */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="border border-slate-700 bg-slate-900/60 rounded-lg p-4">
          <div className="text-xs text-slate-400 mb-1">총 검증</div>
          <div className="text-2xl font-bold text-slate-200">
            {metrics.totalValidations.toLocaleString()}
          </div>
          <div className="mt-2">
            <Chip
              label={`실패: ${metrics.totalFailures.toLocaleString()} (${failureRate}%)`}
              tone={metrics.totalFailures > 0 ? 'error' : 'idle'}
            />
          </div>
        </div>

        <div className="border border-slate-700 bg-slate-900/60 rounded-lg p-4">
          <div className="text-xs text-slate-400 mb-1">자동 복구</div>
          <div className="text-2xl font-bold text-slate-200">
            {metrics.autofixAttempts.toLocaleString()}
          </div>
          <div className="mt-2">
            <Chip
              label={`성공: ${metrics.autofixSuccesses.toLocaleString()} (${autofixRate}%)`}
              tone={parseFloat(autofixRate) > 50 ? 'running' : 'warning'}
            />
          </div>
        </div>

        <div className="border border-slate-700 bg-slate-900/60 rounded-lg p-4">
          <div className="text-xs text-slate-400 mb-1">재시도</div>
          <div className="text-2xl font-bold text-slate-200">
            {metrics.retryAttempts.toLocaleString()}
          </div>
          <div className="mt-2">
            <Chip
              label={`성공: ${metrics.retrySuccesses.toLocaleString()} (${retryRate}%)`}
              tone={parseFloat(retryRate) > 50 ? 'running' : 'warning'}
            />
          </div>
        </div>
      </div>

      {/* Error breakdown */}
      {Object.keys(metrics.byErrorCode).length > 0 && (
        <div className="border border-slate-700 bg-slate-900/60 rounded-lg p-4">
          <h3 className="text-sm font-medium text-slate-300 mb-3">
            오류 유형별 분석
          </h3>
          <div className="space-y-2">
            {Object.entries(metrics.byErrorCode)
              .sort(([, a], [, b]) => b - a)
              .map(([code, count]) => (
                <div
                  key={code}
                  className="flex items-center justify-between py-2 border-b border-slate-800 last:border-0"
                >
                  <span className="text-sm text-slate-300">{code}</span>
                  <div className="flex items-center gap-2">
                    <span className="text-sm font-medium text-slate-200">
                      {count.toLocaleString()}
                    </span>
                    <div className="text-xs text-slate-400">
                      ({((count / metrics.totalFailures) * 100).toFixed(1)}%)
                    </div>
                  </div>
                </div>
              ))}
          </div>
        </div>
      )}
    </div>
  )
}

export default ValidationMetricsDisplay
