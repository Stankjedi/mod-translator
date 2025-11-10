import { create } from "zustand";
import { cancelJob, startJob, subscribe, type Progress } from "../lib/ipc";

type State = {
  status: "idle" | "running" | "stopping";
  total: number;
  done: number;
  file?: string;
  unlisten?: () => void;
  error?: string;
};

type Actions = {
  start: (files: string[], from: string, to: string) => Promise<void>;
  stop: () => Promise<void>;
  dispose: () => void;
};

export const useTranslation = create<State & Actions>((set, get) => ({
  status: "idle",
  total: 0,
  done: 0,

  async start(files, from, to) {
    if (get().status !== "idle") return;

    const un = await subscribe({
      onStarted: ({ total }) => set({ status: "running", total, done: 0, error: undefined }),
      onProgress: (p: Progress) => set({ done: p.done, file: p.file }),
      onStopping: () => set({ status: "stopping" }),
      onCancelled: () => set({ status: "idle" }),
      onFinished: ({ total, done }) => set({ status: "idle", total, done }),
      onError: (e) => set({ error: e.error })
    });

    set({ unlisten: un });
    await startJob(files, from, to);
  },

  async stop() {
    if (get().status === "running") set({ status: "stopping" });
    await cancelJob();
  },

  dispose() {
    get().unlisten?.();
    set({ unlisten: undefined });
  }
}));
