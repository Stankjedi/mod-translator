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

  try {
    const raw = window.localStorage.getItem(STORAGE_KEY)
    if (!raw) {
      return {}
    }

    const parsed = JSON.parse(raw) as Record<string, string>
    return Object.fromEntries(
      Object.entries(parsed)
        .filter(([, value]) => typeof value === 'string' && value.trim().length > 0)
        .map(([key, value]) => [key, value.trim()]),
    )
  } catch (error) {
    console.warn('API 키를 불러오는 중 문제가 발생했습니다.', error)
    return {}
  }
}

export function persistApiKeys(map: ApiKeyMap) {
  if (!isStorageAvailable()) {
    throw new Error('localStorage is not available')
  }

  const sanitizedEntries = Object.entries(map)
    .filter(([, value]) => typeof value === 'string' && value.trim().length > 0)
    .map(([key, value]) => [key, value.trim()])

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

export function maskApiKey(value: string): string {
  const trimmed = value.trim()
  if (!trimmed) {
    return ''
  }

  if (trimmed.length <= 4) {
    return `${trimmed}${'*'.repeat(4)}`
  }

  const prefix = trimmed.slice(0, 4)
  const suffix = trimmed.slice(-2)
  const maskLength = Math.max(trimmed.length - 6, 4)
  return `${prefix}${'*'.repeat(maskLength)}${suffix}`
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
