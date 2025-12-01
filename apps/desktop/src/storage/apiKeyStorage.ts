import { Client, Stronghold } from "@tauri-apps/plugin-stronghold";
import { appDataDir } from "@tauri-apps/api/path";

export type ApiKeyMap = Partial<Record<string, string>>;

const PROVIDERS = ["gemini", "openai", "anthropic", "xai"] as const;
const CLIENT_NAME = "mod-translator-api-keys";
const VAULT_KEY = "mod-translator-stronghold-key-v1";

// Stronghold 싱글톤 캐시
let strongholdInstance: Stronghold | null = null;
let clientInstance: Client | null = null;

/**
 * Stronghold 인스턴스 초기화 및 반환
 */
async function getStrongholdClient(): Promise<Client> {
  if (clientInstance) {
    return clientInstance;
  }

  const vaultPath = `${await appDataDir()}secrets.stronghold`;
  strongholdInstance = await Stronghold.load(vaultPath, VAULT_KEY);

  try {
    clientInstance = await strongholdInstance.loadClient(CLIENT_NAME);
  } catch {
    clientInstance = await strongholdInstance.createClient(CLIENT_NAME);
  }

  return clientInstance;
}

/**
 * Stronghold에서 API 키 로드
 * Tauri 환경이 아닌 경우 localStorage로 폴백
 */
export async function loadApiKeys(): Promise<ApiKeyMap> {
  const result: ApiKeyMap = {};

  // Tauri 환경 확인
  if (typeof window !== "undefined" && "__TAURI__" in window) {
    try {
      const client = await getStrongholdClient();
      const store = client.getStore();

      for (const provider of PROVIDERS) {
        try {
          const data = await store.get(`api_key_${provider}`);
          if (data && data.length > 0) {
            result[provider] = new TextDecoder().decode(new Uint8Array(data));
          }
        } catch (error) {
          console.error(`Failed to load API key for ${provider}:`, error);
        }
      }
      return result;
    } catch (error) {
      console.error("Failed to initialize Stronghold:", error);
      // Stronghold 실패 시 localStorage 폴백
      return loadApiKeysFromLocalStorage();
    }
  }

  // 폴백: localStorage (개발/웹 환경)
  return loadApiKeysFromLocalStorage();
}

/**
 * Stronghold에 API 키 저장
 * Tauri 환경이 아닌 경우 localStorage로 폴백
 */
export async function persistApiKeys(map: ApiKeyMap): Promise<void> {
  // Tauri 환경 확인
  if (typeof window !== "undefined" && "__TAURI__" in window) {
    try {
      const client = await getStrongholdClient();
      const store = client.getStore();

      for (const [provider, key] of Object.entries(map)) {
        const storeKey = `api_key_${provider}`;
        if (key && key.trim().length > 0) {
          const data = Array.from(new TextEncoder().encode(key.trim()));
          await store.insert(storeKey, data);
        } else {
          await store.remove(storeKey);
        }
      }

      // 변경사항 저장
      if (strongholdInstance) {
        await strongholdInstance.save();
      }
      return;
    } catch (error) {
      console.error("Failed to save API keys to Stronghold:", error);
      throw error;
    }
  }

  // 폴백: localStorage (개발/웹 환경)
  persistApiKeysToLocalStorage(map);
}

export function maskApiKey(key: string | undefined | null): string {
  if (!key || key.length === 0) return "";
  if (key.length <= 8) {
    if (key.length <= 2) return "*".repeat(key.length);
    return key[0] + "*".repeat(key.length - 2) + key[key.length - 1];
  }
  const head = key.slice(0, 4);
  const tail = key.slice(-2);
  const maskCount = Math.max(1, key.length - 6);
  return `${head}${"*".repeat(maskCount)}${tail}`;
}

// ===== LocalStorage 폴백 (개발/웹 환경용) =====

const STORAGE_KEY = "mod_translator_api_keys_v1";

function isStorageAvailable() {
  return (
    typeof window !== "undefined" && typeof window.localStorage !== "undefined"
  );
}

function loadApiKeysFromLocalStorage(): ApiKeyMap {
  if (!isStorageAvailable()) {
    return {};
  }

  const raw = window.localStorage.getItem(STORAGE_KEY);
  if (!raw) {
    return {};
  }

  try {
    const parsed = JSON.parse(raw) as Record<string, string | undefined>;

    return Object.entries(parsed).reduce((acc, [provider, value]) => {
      if (typeof value === "string") {
        const cleaned = value.trim();
        if (cleaned.length > 0) {
          acc[provider] = cleaned;
        }
      }
      return acc;
    }, {} as ApiKeyMap);
  } catch {
    return {};
  }
}

function persistApiKeysToLocalStorage(map: ApiKeyMap) {
  if (!isStorageAvailable()) {
    throw new Error("localStorage is not available");
  }

  const sanitizedEntries = Object.entries(map).reduce<[string, string][]>(
    (acc, [key, value]) => {
      if (typeof value === "string") {
        const trimmed = value.trim();
        if (trimmed.length > 0) {
          acc.push([key, trimmed]);
        }
      }
      return acc;
    },
    [],
  );

  try {
    if (sanitizedEntries.length === 0) {
      window.localStorage.removeItem(STORAGE_KEY);
    } else {
      window.localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify(Object.fromEntries(sanitizedEntries)),
      );
    }
  } catch (error) {
    console.error("API 키를 저장하는 중 문제가 발생했습니다.", error);
    throw error;
  }
}
