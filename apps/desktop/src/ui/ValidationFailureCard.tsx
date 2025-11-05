import { useState, type ReactNode } from 'react'
import type {
  ValidationFailureReport,
  ValidationErrorCode,
  RecoveryStep,
} from '../types/core'
import Chip from './Chip'

interface ValidationFailureCardProps {
  report: ValidationFailureReport
  onRetry?: (report: ValidationFailureReport) => void
  onDismiss?: (report: ValidationFailureReport) => void
}

const errorCodeLabels: Record<ValidationErrorCode, string> = {
  PLACEHOLDER_MISMATCH: '자리표시자 불일치',
  PAIR_UNBALANCED: '태그 쌍 불균형',
  FORMAT_TOKEN_MISSING: '포맷 토큰 누락',
  XML_MALFORMED_AFTER_RESTORE: 'XML 구조 손상',
  RETRY_FAILED: '재시도 실패',
}

const recoveryStepLabels: Record<RecoveryStep, string> = {
  REINJECT_MISSING_PROTECTED: '누락 토큰 재주입',
  PAIR_BALANCE_CHECK: '태그 쌍 균형 검사',
  REMOVE_EXCESS_TOKENS: '과잉 토큰 제거',
  CORRECT_FORMAT_TOKENS: '포맷 토큰 수정',
  PRESERVE_PERCENT_BINDING: '%기호 결합 유지',
}

function TokenDiff({
  label,
  expected,
  found,
}: {
  label: string
  expected: string[]
  found: string[]
}) {
  const missing = expected.filter((token) => !found.includes(token))
  const extra = found.filter((token) => !expected.includes(token))
  const matched = expected.filter((token) => found.includes(token))

  if (missing.length === 0 && extra.length === 0) {
    return null
  }

  return (
    <div className="space-y-2">
      <h4 className="text-sm font-medium text-slate-300">{label}</h4>
      {matched.length > 0 && (
        <div>
          <div className="text-xs text-slate-400 mb-1">일치:</div>
          <div className="flex flex-wrap gap-1">
            {matched.map((token, idx) => (
              <code
                key={idx}
                className="px-2 py-1 bg-emerald-500/10 text-emerald-300 border border-emerald-500/30 rounded text-xs"
              >
                {token}
              </code>
            ))}
          </div>
        </div>
      )}
      {missing.length > 0 && (
        <div>
          <div className="text-xs text-rose-400 mb-1">누락:</div>
          <div className="flex flex-wrap gap-1">
            {missing.map((token, idx) => (
              <code
                key={idx}
                className="px-2 py-1 bg-rose-500/10 text-rose-300 border border-rose-500/30 rounded text-xs"
              >
                {token}
              </code>
            ))}
          </div>
        </div>
      )}
      {extra.length > 0 && (
        <div>
          <div className="text-xs text-amber-400 mb-1">추가:</div>
          <div className="flex flex-wrap gap-1">
            {extra.map((token, idx) => (
              <code
                key={idx}
                className="px-2 py-1 bg-amber-500/10 text-amber-300 border border-amber-500/30 rounded text-xs"
              >
                {token}
              </code>
            ))}
          </div>
        </div>
      )}
    </div>
  )
}

function CopyButton({ text, label }: { text: string; label: string }) {
  const [copied, setCopied] = useState(false)

  const handleCopy = async () => {
    try {
      await navigator.clipboard.writeText(text)
      setCopied(true)
      setTimeout(() => setCopied(false), 2000)
    } catch (err) {
      console.error('Failed to copy:', err)
    }
  }

  return (
    <button
      onClick={handleCopy}
      className="px-3 py-1.5 text-xs font-medium text-slate-300 bg-slate-800 border border-slate-700 rounded hover:bg-slate-700 transition-colors"
    >
      {copied ? '✓ 복사됨' : `${label} 복사`}
    </button>
  )
}

export function ValidationFailureCard({
  report,
  onRetry,
  onDismiss,
}: ValidationFailureCardProps) {
  const [showSource, setShowSource] = useState(true)
  const [showCandidate, setShowCandidate] = useState(true)
  const [expanded, setExpanded] = useState(false)

  return (
    <div className="border border-rose-500/30 bg-rose-500/5 rounded-lg p-4 space-y-4">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div className="space-y-1 flex-1">
          <div className="flex items-center gap-2">
            <Chip label={errorCodeLabels[report.code]} tone="error" />
            {report.autofix.applied && (
              <Chip label="자동 복구 시도됨" tone="warning" variant="status" />
            )}
            {report.retry.attempted && (
              <Chip
                label={report.retry.success ? '재시도 성공' : '재시도 실패'}
                tone={report.retry.success ? 'running' : 'error'}
                variant="status"
              />
            )}
          </div>
          <div className="text-sm text-slate-300">
            <span className="font-medium">{report.key}</span>
            <span className="text-slate-500 ml-2">
              {report.file}:{report.line}
            </span>
          </div>
        </div>
        {onDismiss && (
          <button
            onClick={() => onDismiss(report)}
            className="text-slate-400 hover:text-slate-200 transition-colors"
          >
            <svg
              className="w-5 h-5"
              fill="none"
              stroke="currentColor"
              viewBox="0 0 24 24"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M6 18L18 6M6 6l12 12"
              />
            </svg>
          </button>
        )}
      </div>

      {/* Toggle buttons */}
      <div className="flex gap-2">
        <button
          onClick={() => setShowSource(!showSource)}
          className={`px-3 py-1.5 text-xs font-medium rounded transition-colors ${
            showSource
              ? 'bg-brand-500/20 text-brand-300 border border-brand-500/30'
              : 'bg-slate-800 text-slate-400 border border-slate-700'
          }`}
        >
          {showSource ? '원문 숨기기' : '원문 보기'}
        </button>
        <button
          onClick={() => setShowCandidate(!showCandidate)}
          className={`px-3 py-1.5 text-xs font-medium rounded transition-colors ${
            showCandidate
              ? 'bg-brand-500/20 text-brand-300 border border-brand-500/30'
              : 'bg-slate-800 text-slate-400 border border-slate-700'
          }`}
        >
          {showCandidate ? '번역 숨기기' : '번역 보기'}
        </button>
        <button
          onClick={() => setExpanded(!expanded)}
          className="px-3 py-1.5 text-xs font-medium rounded transition-colors bg-slate-800 text-slate-400 border border-slate-700"
        >
          {expanded ? '상세 숨기기' : '상세 보기'}
        </button>
      </div>

      {/* Source line */}
      {showSource && (
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <h4 className="text-sm font-medium text-slate-300">원문 라인</h4>
            {report.uiHint.copyButtons && (
              <CopyButton text={report.sourceLine} label="원문" />
            )}
          </div>
          <pre className="p-3 bg-slate-900 border border-slate-700 rounded text-xs text-slate-300 overflow-x-auto">
            {report.sourceLine}
          </pre>
          {expanded && (
            <>
              <h4 className="text-sm font-medium text-slate-400">
                전처리된 원문 (토큰 치환 후)
              </h4>
              <pre className="p-3 bg-slate-900 border border-slate-700 rounded text-xs text-slate-300 overflow-x-auto">
                {report.preprocessedSource}
              </pre>
            </>
          )}
        </div>
      )}

      {/* Candidate line */}
      {showCandidate && (
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <h4 className="text-sm font-medium text-slate-300">번역 시도</h4>
            {report.uiHint.copyButtons && (
              <CopyButton text={report.candidateLine} label="번역" />
            )}
          </div>
          <pre className="p-3 bg-slate-900 border border-slate-700 rounded text-xs text-slate-300 overflow-x-auto">
            {report.candidateLine}
          </pre>
        </div>
      )}

      {/* Token diff */}
      {expanded && (
        <div className="space-y-3 pt-2 border-t border-slate-700">
          <TokenDiff
            label="보호 토큰 비교"
            expected={report.expectedProtected}
            found={report.foundProtected}
          />
          {(report.expectedFormat.length > 0 ||
            report.foundFormat.length > 0) && (
            <TokenDiff
              label="포맷 토큰 비교"
              expected={report.expectedFormat}
              found={report.foundFormat}
            />
          )}
        </div>
      )}

      {/* Auto-recovery info */}
      {report.autofix.applied && expanded && (
        <div className="space-y-2 pt-2 border-t border-slate-700">
          <h4 className="text-sm font-medium text-amber-300">
            적용된 자동 복구 단계
          </h4>
          <div className="flex flex-wrap gap-1">
            {report.autofix.steps.map((step, idx) => (
              <span
                key={idx}
                className="px-2 py-1 bg-amber-500/10 text-amber-300 border border-amber-500/30 rounded text-xs"
              >
                {recoveryStepLabels[step]}
              </span>
            ))}
          </div>
        </div>
      )}

      {/* Actions */}
      {onRetry && (
        <div className="flex gap-2 pt-2 border-t border-slate-700">
          <button
            onClick={() => onRetry(report)}
            className="px-4 py-2 text-sm font-medium text-white bg-brand-500 hover:bg-brand-600 rounded transition-colors"
          >
            자동 복구로 재시도
          </button>
        </div>
      )}
    </div>
  )
}

interface ValidationFailureListProps {
  failures: ValidationFailureReport[]
  onRetry?: (report: ValidationFailureReport) => void
  onDismiss?: (report: ValidationFailureReport) => void
  onDownloadJson?: () => void
  onDownloadCsv?: () => void
}

export function ValidationFailureList({
  failures,
  onRetry,
  onDismiss,
  onDownloadJson,
  onDownloadCsv,
}: ValidationFailureListProps) {
  if (failures.length === 0) {
    return (
      <div className="text-center py-12 text-slate-400">
        검증 실패 항목이 없습니다
      </div>
    )
  }

  return (
    <div className="space-y-4">
      {/* Header with download buttons */}
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold text-slate-200">
          검증 실패 목록 ({failures.length}개)
        </h2>
        <div className="flex gap-2">
          {onDownloadJson && (
            <button
              onClick={onDownloadJson}
              className="px-3 py-1.5 text-xs font-medium text-slate-300 bg-slate-800 border border-slate-700 rounded hover:bg-slate-700 transition-colors"
            >
              JSON 다운로드
            </button>
          )}
          {onDownloadCsv && (
            <button
              onClick={onDownloadCsv}
              className="px-3 py-1.5 text-xs font-medium text-slate-300 bg-slate-800 border border-slate-700 rounded hover:bg-slate-700 transition-colors"
            >
              CSV 다운로드
            </button>
          )}
        </div>
      </div>

      {/* Failure cards */}
      <div className="space-y-3">
        {failures.map((report, idx) => (
          <ValidationFailureCard
            key={`${report.file}-${report.line}-${idx}`}
            report={report}
            onRetry={onRetry}
            onDismiss={onDismiss}
          />
        ))}
      </div>
    </div>
  )
}

export default ValidationFailureCard
