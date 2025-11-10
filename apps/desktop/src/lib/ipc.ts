import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

export type Progress = { total: number; done: number; file?: string };

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
  onCancelled?: (p: any) => void;
  onFinished?: (p: { total: number; done: number }) => void;
  onError?: (e: { file?: string; error: string }) => void;
}) {
  const unsubs: UnlistenFn[] = [];

  const bind = async (name: string, fn: (e: any) => void) => {
    unsubs.push(await listen(name, (ev) => fn(ev.payload)));
  };

  const promises = [
    handlers.onStarted && bind("translate:started", handlers.onStarted),
    handlers.onProgress && bind("translate:progress", handlers.onProgress),
    handlers.onStopping && bind("translate:stopping", handlers.onStopping),
    handlers.onCancelled && bind("translate:cancelled", handlers.onCancelled),
    handlers.onFinished && bind("translate:finished", handlers.onFinished),
    handlers.onError && bind("translate:error", handlers.onError)
  ];

  return Promise.all(promises).then(() => () => unsubs.forEach((u) => u()));
}
