import type { ReactNode } from 'react'
import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from 'react'
import { invoke } from '@tauri-apps/api/core'
import type {
  LibraryEntry,
  LibraryScanResponse,
  PolicyBanner,
  SteamPathResponse,
} from '../types/core'

export interface SteamPathInfo {
  path: string | null
  note: string
}

interface LibraryContextValue {
  policyBanner: PolicyBanner | null
  libraries: LibraryEntry[]
  isScanning: boolean
  scanLibrary: (explicitPath?: string) => Promise<boolean>
  error: string | null
  steamPath: SteamPathInfo | null
  detectSteamPath: () => Promise<SteamPathInfo | null>
}

const LibraryContext = createContext<LibraryContextValue | undefined>(undefined)

const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export function LibraryProvider({ children }: { children: ReactNode }) {
  const [policyBanner, setPolicyBanner] = useState<PolicyBanner | null>(null)
  const [libraries, setLibraries] = useState<LibraryEntry[]>([])
  const [isScanning, setIsScanning] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [steamPath, setSteamPath] = useState<SteamPathInfo | null>(null)

  const detectSteamPath = useCallback(async () => {
    if (!isTauri()) {
      const fallback: SteamPathInfo = {
        path: null,
        note: '이 기능은 데스크톱(Tauri) 환경에서만 사용할 수 있습니다.',
      }
      setSteamPath(fallback)
      setError('로컬 애플리케이션에서 실행 중인지 확인해 주세요.')
      return fallback
    }

    try {
      const response = await invoke<SteamPathResponse>('detect_steam_path')
      const info: SteamPathInfo = {
        path: response.path,
        note: response.note,
      }
      setSteamPath(info)
      setError(null)
      return info
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      setError(message)
      return null
    }
  }, [])

  const scanLibrary = useCallback(
    async (explicitPath?: string) => {
      if (!isTauri()) {
        setLibraries([])
        setPolicyBanner(null)
        setError('라이브러리 스캔은 Tauri 환경에서만 지원됩니다.')
        return false
      }

      setIsScanning(true)
      try {
        const response = await invoke<LibraryScanResponse>('scan_steam_library', {
          explicit_path: explicitPath ?? null,
        })
        setLibraries(response.libraries)
        setPolicyBanner(response.policy_banner)
        setError(null)
        return true
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err)
        setError(message)
        setLibraries([])
        return false
      } finally {
        setIsScanning(false)
      }
    },
    [],
  )

  useEffect(() => {
    ;(async () => {
      const info = await detectSteamPath()
      if (info?.path) {
        await scanLibrary(info.path)
      }
    })()
  }, [detectSteamPath, scanLibrary])

  const value = useMemo(
    () => ({
      policyBanner,
      libraries,
      isScanning,
      scanLibrary,
      error,
      steamPath,
      detectSteamPath,
    }),
    [policyBanner, libraries, isScanning, scanLibrary, error, steamPath, detectSteamPath],
  )

  return <LibraryContext.Provider value={value}>{children}</LibraryContext.Provider>
}

export function useLibraryContext() {
  const context = useContext(LibraryContext)
  if (!context) {
    throw new Error('LibraryContext는 LibraryProvider 내에서만 사용할 수 있습니다.')
  }
  return context
}
