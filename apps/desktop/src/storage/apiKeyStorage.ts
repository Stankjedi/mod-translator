import type { ProviderAuth } from '../types/core'

const STORAGE_KEY = 'mod_translator_api_keys_v1'

export type ApiKeyMap = Partial<Record<string, string>>

function isStorageAvailable() {
  return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined'
}

export function loadApiKeys(): ApiKeyMap {
  if (!isStorageAvailable()) {
    return {}
  }

  const raw = window.localStorage.getItem(STORAGE_KEY)
  if (!raw) {
    return {}
  }

  try {
    const parsed = JSON.parse(raw) as Record<string, string | undefined>

    return Object.entries(parsed).reduce((acc, [provider, value]) => {
      if (typeof value === 'string') {
        const cleaned = value.trim()
        if (cleaned.length > 0) {
          acc[provider] = cleaned
        }
      }
      return acc
    }, {} as ApiKeyMap)
  } catch {
    return {}
  }
}

export function persistApiKeys(map: ApiKeyMap) {
  if (!isStorageAvailable()) {
    throw new Error('localStorage is not available')
  }

  const sanitizedEntries = Object.entries(map).reduce<[string, string][]>((acc, [key, value]) => {
    if (typeof value === 'string') {
      const trimmed = value.trim()
      if (trimmed.length > 0) {
        acc.push([key, trimmed])
      }
    }
    return acc
  }, [])

  try {
    if (sanitizedEntries.length === 0) {
      window.localStorage.removeItem(STORAGE_KEY)
    } else {
      window.localStorage.setItem(STORAGE_KEY, JSON.stringify(Object.fromEntries(sanitizedEntries)))
    }
  } catch (error) {
    console.error('API 키를 저장하는 중 문제가 발생했습니다.', error)
    throw error
  }
}

export function maskApiKey(key: string | undefined | null): string {
  if (!key || key.length === 0) return ''
  if (key.length <= 8) {
    if (key.length <= 2) return '*'.repeat(key.length)
    return key[0] + '*'.repeat(key.length - 2) + key[key.length - 1]
  }
  const head = key.slice(0, 4)
  const tail = key.slice(-2)
  const maskCount = Math.max(1, key.length - 6)
  return `${head}${'*'.repeat(maskCount)}${tail}`
}

export function getStoredProviderAuth(): ProviderAuth {
  const keys = loadApiKeys()
  return {
    gemini: keys.gemini ?? null,
    gpt: keys.gpt ?? null,
    claude: keys.claude ?? null,
    grok: keys.grok ?? null,
  }
}
