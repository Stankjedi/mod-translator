const POLICY_KEY = 'mod_translator_policy_ack_v1'

function isStorageAvailable() {
  return typeof window !== 'undefined' && typeof window.localStorage !== 'undefined'
}

export function getPolicyAcknowledged(): boolean {
  if (!isStorageAvailable()) {
    return false
  }

  try {
    const value = window.localStorage.getItem(POLICY_KEY)
    return value === 'true'
  } catch (error) {
    console.warn('정책 동의 상태를 불러오는 중 문제가 발생했습니다.', error)
    return false
  }
}

export function setPolicyAcknowledged(value: boolean) {
  if (!isStorageAvailable()) {
    return
  }

  try {
    window.localStorage.setItem(POLICY_KEY, value ? 'true' : 'false')
  } catch (error) {
    console.warn('정책 동의 상태를 저장하는 중 문제가 발생했습니다.', error)
  }
}
