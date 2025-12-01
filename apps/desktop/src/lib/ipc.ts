import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Progress = { total: number; done: number; file?: string };

// 백엔드에서 전달되는 에러 구조
export interface TranslationError {
  file?: string;
  error: string;
  error_type?:
    | "network"
    | "api_key_invalid"
    | "rate_limit"
    | "format_error"
    | "file_not_found"
    | "permission_denied"
    | "timeout"
    | "unknown";
  retry_after_ms?: number;
}

export async function startJob(files: string[], from: string, to: string) {
  if (!files?.length) throw new Error("no files");
  return invoke("cmd_start", { files, from, to });
}

export async function cancelJob() {
  return invoke("cmd_cancel");
}

export function subscribe(handlers: {
  onStarted?: (p: { total: number }) => void;
  onProgress?: (p: Progress) => void;
  onStopping?: () => void;
  onCancelled?: (p: unknown) => void;
  onFinished?: (p: { total: number; done: number }) => void;
  onError?: (e: TranslationError) => void;
}) {
  const unsubs: UnlistenFn[] = [];

  const bind = async <T>(name: string, fn: (e: T) => void) => {
    unsubs.push(await listen(name, (ev) => fn(ev.payload as T)));
  };

  const promises = [
    handlers.onStarted &&
      bind<{ total: number }>("translate:started", handlers.onStarted),
    handlers.onProgress &&
      bind<Progress>("translate:progress", handlers.onProgress),
    handlers.onStopping &&
      bind<void>("translate:stopping", handlers.onStopping),
    handlers.onCancelled &&
      bind<unknown>("translate:cancelled", handlers.onCancelled),
    handlers.onFinished &&
      bind<{ total: number; done: number }>(
        "translate:finished",
        handlers.onFinished,
      ),
    handlers.onError &&
      bind<TranslationError>("translate:error", handlers.onError),
  ];

  return Promise.all(promises).then(() => () => unsubs.forEach((u) => u()));
}
