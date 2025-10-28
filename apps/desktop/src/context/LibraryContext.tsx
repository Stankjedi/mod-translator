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

interface LibraryContextValue {
  policyBanner: PolicyBanner | null
  libraries: LibraryEntry[]
  isScanning: boolean
  scanLibrary: (explicitPath?: string) => Promise<void>
  error: string | null
  steamPath: string | null
  detectSteamPath: () => Promise<string | null>
}

const LibraryContext = createContext<LibraryContextValue | undefined>(undefined)

const isTauri = () => typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window

export function LibraryProvider({ children }: { children: ReactNode }) {
  const [policyBanner, setPolicyBanner] = useState<PolicyBanner | null>(null)
  const [libraries, setLibraries] = useState<LibraryEntry[]>([])
  const [isScanning, setIsScanning] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [steamPath, setSteamPath] = useState<string | null>(null)

  const detectSteamPath = useCallback(async () => {
    if (!isTauri()) {
      setSteamPath(null)
      setError('로컬 애플리케이션에서 실행 중인지 확인해 주세요.')
      return null
    }

    try {
      const response = await invoke<SteamPathResponse>('detect_steam_path')
      setSteamPath(response.path ?? null)
      setError(null)
      return response.path ?? null
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err)
      setSteamPath(null)
      setError(message)
      return null
    }
  }, [])

  const scanLibrary = useCallback(
    async (explicitPath?: string) => {
      if (!isTauri()) {
        setLibraries([])
        setPolicyBanner(null)
        const message = '라이브러리 스캔은 Tauri 환경에서만 지원됩니다.'
        setError(message)
        throw new Error(message)
      }

      setIsScanning(true)
      try {
        const response = await invoke<LibraryScanResponse>('scan_steam_library', {
          explicit_path: explicitPath ?? null,
        })
        setLibraries(response.libraries)
        setPolicyBanner(response.policy_banner)
        setError(null)
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err)
        setError(message)
        setLibraries([])
        throw err instanceof Error ? err : new Error(message)
      } finally {
        setIsScanning(false)
      }
    },
    [],
  )

  useEffect(() => {
    ;(async () => {
      const path = await detectSteamPath()
      if (path) {
        try {
          await scanLibrary(path)
        } catch (error) {
          console.error('자동 스캔 중 오류가 발생했습니다.', error)
        }
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
