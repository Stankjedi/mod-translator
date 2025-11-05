import { useState } from 'react'
import type { ValidatorConfig } from '../types/core'

interface ValidatorSettingsPanelProps {
  config: ValidatorConfig
  onChange: (config: ValidatorConfig) => void
}

export function ValidatorSettingsPanel({
  config,
  onChange,
}: ValidatorSettingsPanelProps) {
  const [localConfig, setLocalConfig] = useState(config)

  const handleChange = (key: keyof ValidatorConfig, value: boolean | number) => {
    const newConfig = { ...localConfig, [key]: value }
    setLocalConfig(newConfig)
    onChange(newConfig)
  }

  return (
    <div className="space-y-6">
      <div>
        <h3 className="text-lg font-semibold text-slate-200 mb-4">
          자리표시자 검증기 설정
        </h3>
        <p className="text-sm text-slate-400 mb-4">
          번역 중 보호 토큰과 포맷 토큰 검증 동작을 구성합니다.
        </p>
      </div>

      {/* Auto-fix settings */}
      <div className="space-y-4">
        <h4 className="text-sm font-medium text-slate-300">자동 복구</h4>

        <label className="flex items-start gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={localConfig.enableAutofix}
            onChange={(e) => handleChange('enableAutofix', e.target.checked)}
            className="mt-1 w-4 h-4 rounded border-slate-700 bg-slate-900 text-brand-500 focus:ring-brand-500 focus:ring-offset-0"
          />
          <div className="flex-1">
            <div className="text-sm font-medium text-slate-200">
              자동 복구 활성화
            </div>
            <div className="text-xs text-slate-400 mt-1">
              검증 실패 시 자동으로 토큰을 복구합니다. 누락된 토큰 재주입, 쌍 균형 조정, 과잉 토큰 제거 등을 수행합니다.
            </div>
          </div>
        </label>

        <label className="flex items-start gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={localConfig.strictPairing}
            onChange={(e) => handleChange('strictPairing', e.target.checked)}
            className="mt-1 w-4 h-4 rounded border-slate-700 bg-slate-900 text-brand-500 focus:ring-brand-500 focus:ring-offset-0"
            disabled={!localConfig.enableAutofix}
          />
          <div className="flex-1">
            <div className="text-sm font-medium text-slate-200">
              엄격한 쌍 검사
            </div>
            <div className="text-xs text-slate-400 mt-1">
              여는 태그와 닫는 태그의 쌍을 엄격하게 검증합니다. 불균형 감지 시 자동으로 균형을 맞춥니다.
            </div>
          </div>
        </label>

        <label className="flex items-start gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={localConfig.preservePercentBinding}
            onChange={(e) =>
              handleChange('preservePercentBinding', e.target.checked)
            }
            className="mt-1 w-4 h-4 rounded border-slate-700 bg-slate-900 text-brand-500 focus:ring-brand-500 focus:ring-offset-0"
            disabled={!localConfig.enableAutofix}
          />
          <div className="flex-1">
            <div className="text-sm font-medium text-slate-200">
              퍼센트 결합 유지
            </div>
            <div className="text-xs text-slate-400 mt-1">
              {'{0}%'}, {'{1}%'} 같은 포맷 토큰과 퍼센트 기호의 결합을 유지합니다.
            </div>
          </div>
        </label>
      </div>

      {/* Retry settings */}
      <div className="space-y-4">
        <h4 className="text-sm font-medium text-slate-300">재시도</h4>

        <label className="flex items-start gap-3 cursor-pointer">
          <input
            type="checkbox"
            checked={localConfig.retryOnFail}
            onChange={(e) => handleChange('retryOnFail', e.target.checked)}
            className="mt-1 w-4 h-4 rounded border-slate-700 bg-slate-900 text-brand-500 focus:ring-brand-500 focus:ring-offset-0"
          />
          <div className="flex-1">
            <div className="text-sm font-medium text-slate-200">
              실패 시 재시도
            </div>
            <div className="text-xs text-slate-400 mt-1">
              자동 복구가 실패하면 향상된 프롬프트로 부분 재시도를 수행합니다.
            </div>
          </div>
        </label>

        <div className="space-y-2">
          <label className="block">
            <div className="text-sm font-medium text-slate-200 mb-2">
              재시도 제한
            </div>
            <input
              type="number"
              min="0"
              max="5"
              value={localConfig.retryLimit}
              onChange={(e) =>
                handleChange('retryLimit', parseInt(e.target.value, 10))
              }
              disabled={!localConfig.retryOnFail}
              className="w-full px-3 py-2 bg-slate-900 border border-slate-700 rounded text-slate-200 focus:outline-none focus:ring-2 focus:ring-brand-500 disabled:opacity-50"
            />
            <div className="text-xs text-slate-400 mt-1">
              각 라인당 최대 재시도 횟수 (권장: 1)
            </div>
          </label>
        </div>
      </div>

      {/* Info box */}
      <div className="border border-brand-500/30 bg-brand-500/5 rounded-lg p-4">
        <div className="flex gap-3">
          <div className="text-brand-400">
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
                d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
              />
            </svg>
          </div>
          <div className="flex-1">
            <div className="text-sm font-medium text-brand-200 mb-1">
              검증기 정보
            </div>
            <div className="text-xs text-brand-300/80 space-y-1">
              <div>
                • 보호 토큰: ⟦MT:TAG:0⟧, ⟦MT:CODE:1⟧ 등 태그와 코드를 보호
              </div>
              <div>
                • 포맷 토큰: {'{0}'}, {'{1}'} 등 서식 자리표시자
              </div>
              <div>
                • 자동 복구: 누락 토큰 재주입, 쌍 균형, 과잉 제거 등 5단계
              </div>
              <div>
                • 실패 보고: 원문/번역/토큰 비교를 포함한 상세 리포트
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Example */}
      <div className="border border-slate-700 bg-slate-900/60 rounded-lg p-4">
        <h4 className="text-sm font-medium text-slate-300 mb-3">예시</h4>
        <div className="space-y-3 text-xs">
          <div>
            <div className="text-slate-400 mb-1">원문:</div>
            <code className="block p-2 bg-slate-950 border border-slate-800 rounded text-slate-300">
              ⟦MT:TAG:0⟧Relative frequency for hookups...⟦MT:TAG:1⟧
            </code>
          </div>
          <div>
            <div className="text-slate-400 mb-1">번역 (토큰 누락):</div>
            <code className="block p-2 bg-slate-950 border border-slate-800 rounded text-rose-300">
              후킹의 상대적 빈도입니다...
            </code>
          </div>
          <div>
            <div className="text-slate-400 mb-1">
              자동 복구 후 (토큰 재주입):
            </div>
            <code className="block p-2 bg-slate-950 border border-slate-800 rounded text-emerald-300">
              ⟦MT:TAG:0⟧후킹의 상대적 빈도입니다...⟦MT:TAG:1⟧
            </code>
          </div>
        </div>
      </div>
    </div>
  )
}

export default ValidatorSettingsPanel
