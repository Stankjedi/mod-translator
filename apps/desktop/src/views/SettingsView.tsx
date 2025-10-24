import { useEffect, useMemo, useState } from 'react'
import { useLibraryContext } from '../context/LibraryContext'
import { useI18n } from '../i18n/ko'

const providers = [
  {
    name: 'Gemini Advanced',
    description: 'Google 기반 컨텍스트 확장 모델을 사용합니다.',
    defaultChecked: true,
  },
  {
    name: 'GPT-4.1 Turbo',
    description: '긴 컨텍스트와 안정적인 번역 품질을 제공합니다.',
    defaultChecked: true,
  },
  {
    name: 'Claude 3.5 Sonnet',
    description: '대사 중심 콘텐츠에 적합한 Anthropic 어댑터입니다.',
    defaultChecked: false,
  },
  {
    name: 'xAI Grok 2',
    description: '신속한 반복 실험에 적합한 실험적 제공자입니다.',
    defaultChecked: false,
  },
]

function SettingsView() {
  const i18n = useI18n()
  const steamTexts = i18n.settings.steam
  const limitTexts = i18n.settings.limits
  const formatDetectedNote = useMemo(
    () =>
      (path: string) =>
        steamTexts.noteDetected.replace('{path}', path),
    [steamTexts.noteDetected],
  )
  const { steamPath, detectSteamPath, scanLibrary, isScanning } = useLibraryContext()
  const [explicitPath, setExplicitPath] = useState(steamPath?.path ?? '')
  const [pathNote, setPathNote] = useState('')
  const [scanStatus, setScanStatus] = useState('')

  useEffect(() => {
    setExplicitPath(steamPath?.path ?? '')

    if (!steamPath) {
      setPathNote('')
      return
    }

    const trimmed = steamPath.path?.trim()
    if (trimmed) {
      setPathNote(formatDetectedNote(trimmed))
    } else {
      setPathNote(steamTexts.noteNotFound)
    }
  }, [steamPath, formatDetectedNote, steamTexts.noteNotFound])

  const handleDetect = async () => {
    setScanStatus('')
    const info = await detectSteamPath()
    if (!info) {
      setScanStatus(steamTexts.noteError)
      return
    }

    const detected = info.path?.trim()
    setExplicitPath(detected ?? '')

    if (detected) {
      setPathNote(formatDetectedNote(detected))
      setScanStatus(steamTexts.scanning)
      const success = await scanLibrary(detected)
      setScanStatus(success ? steamTexts.noteDone : steamTexts.noteError)
    } else {
      setPathNote(steamTexts.noteNotFound)
      setScanStatus('')
    }
  }

  const handleScan = async () => {
    const trimmed = explicitPath.trim()
    if (!trimmed) {
      setPathNote(steamTexts.noteEmpty)
      setScanStatus('')
      return
    }

    setPathNote(formatDetectedNote(trimmed))
    setScanStatus(steamTexts.scanning)
    const success = await scanLibrary(trimmed)
    setScanStatus(success ? steamTexts.noteDone : steamTexts.noteError)
  }

  return (
    <div className="space-y-8">
      <header>
        <h2 className="text-xl font-semibold text-white">작업 공간 설정</h2>
        <p className="text-sm text-slate-400">
          번역 엔진, Steam 연동, 그리고 처리량 제한을 조정할 수 있습니다. 이 화면은 Rust 백엔드와의 연계를 고려한 자리 표시자 UI입니다.
        </p>
      </header>

      <form className="space-y-6">
        <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30">
          <h3 className="text-lg font-semibold text-white">AI 제공자</h3>
          <p className="mt-1 text-sm text-slate-400">번역 작업에 사용할 모델을 선택하세요.</p>
          <div className="mt-4 grid gap-4 sm:grid-cols-2">
            {providers.map((provider) => (
              <label
                key={provider.name}
                className="flex items-center gap-3 rounded-xl border border-slate-800/60 bg-slate-950/60 p-4"
              >
                <input
                  type="checkbox"
                  defaultChecked={provider.defaultChecked}
                  className="h-4 w-4 rounded border-slate-700 bg-slate-900"
                />
                <span>
                  <span className="block text-sm font-medium text-white">{provider.name}</span>
                  <span className="text-xs text-slate-400">{provider.description}</span>
                </span>
              </label>
            ))}
          </div>
        </section>

        <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30">
          <h3 className="text-lg font-semibold text-white">{steamTexts.title}</h3>
          <div className="mt-4 space-y-4">
            <label className="block text-sm font-medium text-slate-300">{steamTexts.pathLabel}</label>
            <input
              type="text"
              value={explicitPath}
              onChange={(event) => setExplicitPath(event.target.value)}
              placeholder="예: C:/Program Files (x86)/Steam"
              className="w-full rounded-xl border border-slate-800 bg-slate-950 px-4 py-3 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
            />
            {pathNote && <p className="text-xs text-slate-400">{pathNote}</p>}
            {scanStatus && <p className="text-xs text-slate-500">{scanStatus}</p>}
            <div className="flex flex-wrap gap-2">
              <button
                type="button"
                onClick={handleDetect}
                className="inline-flex items-center justify-center rounded-xl bg-slate-800 px-4 py-2 text-sm font-semibold text-white transition hover:bg-slate-700"
              >
                {steamTexts.detect}
              </button>
              <button
                type="button"
                onClick={handleScan}
                disabled={isScanning}
                className="inline-flex items-center justify-center rounded-xl bg-brand-600 px-4 py-2 text-sm font-semibold text-white shadow shadow-brand-600/40 transition hover:bg-brand-500 disabled:cursor-not-allowed disabled:opacity-60"
              >
                {isScanning ? steamTexts.scanning : steamTexts.scan}
              </button>
            </div>
          </div>
        </section>

        <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30">
          <h3 className="text-lg font-semibold text-white">{limitTexts.title}</h3>
          <p className="mt-1 text-sm text-slate-400">번역 큐와 토큰 버킷을 조정해 공급자 제한을 준수하세요.</p>
          <div className="mt-4 grid gap-4 md:grid-cols-2 xl:grid-cols-4">
            <label className="flex flex-col gap-2 text-sm text-slate-300">
              <span>{limitTexts.concurrency}</span>
              <input
                type="number"
                defaultValue={3}
                min={1}
                className="rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
              />
              <p className="text-xs text-slate-500">{limitTexts.hints.concurrency}</p>
            </label>
            <label className="flex flex-col gap-2 text-sm text-slate-300">
              <span>{limitTexts.workers}</span>
              <input
                type="number"
                defaultValue={2}
                min={1}
                className="rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
              />
              <p className="text-xs text-slate-500">{limitTexts.hints.workers}</p>
            </label>
            <label className="flex flex-col gap-2 text-sm text-slate-300">
              <span>{limitTexts.bucket}</span>
              <input
                type="number"
                defaultValue={5}
                min={1}
                className="rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
              />
              <p className="text-xs text-slate-500">{limitTexts.hints.bucket}</p>
            </label>
            <label className="flex flex-col gap-2 text-sm text-slate-300">
              <span>{limitTexts.refillMs}</span>
              <input
                type="number"
                defaultValue={750}
                min={100}
                step={50}
                className="rounded-xl border border-slate-800 bg-slate-950 px-3 py-2 text-sm text-slate-100 focus:border-brand-500 focus:ring-brand-500"
              />
              <p className="text-xs text-slate-500">{limitTexts.hints.refillMs}</p>
            </label>
          </div>
        </section>

        <section className="rounded-2xl border border-slate-800/60 bg-slate-900/60 p-6 shadow-inner shadow-black/30">
          <h3 className="text-lg font-semibold text-white">번역 규칙 및 로깅</h3>
          <div className="mt-4 space-y-4 text-sm text-slate-300">
            <label className="flex items-center justify-between gap-3">
              <span>백엔드 상세 로그 남기기</span>
              <input type="checkbox" className="h-4 w-4 rounded border-slate-700 bg-slate-900" />
            </label>
            <label className="flex items-center justify-between gap-3">
              <span>플레이스홀더 일치 검증 강제</span>
              <input type="checkbox" defaultChecked className="h-4 w-4 rounded border-slate-700 bg-slate-900" />
            </label>
            <label className="flex items-center justify-between gap-3">
              <span>DLL 리소스 우선 처리 (Mono.Cecil)</span>
              <input type="checkbox" defaultChecked className="h-4 w-4 rounded border-slate-700 bg-slate-900" />
            </label>
            <label className="flex items-center justify-between gap-3">
              <span>품질 샘플링(5%) 수행</span>
              <input type="checkbox" defaultChecked className="h-4 w-4 rounded border-slate-700 bg-slate-900" />
            </label>
          </div>
        </section>
      </form>
    </div>
  )
}

export default SettingsView
