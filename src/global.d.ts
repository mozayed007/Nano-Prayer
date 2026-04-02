import type { UnlistenFn } from "$lib/desktop/api";

declare global {
  interface Window {
    electronAPI?: {
      invoke: (command: string, args?: Record<string, unknown>) => Promise<unknown>;
      listen: (event: string, cb: (payload: unknown) => void) => UnlistenFn;
      window: {
        hide: () => Promise<void>;
        show: () => Promise<void>;
        focus: () => Promise<void>;
        isVisible: () => Promise<boolean>;
      };
    };
  }
}

export {};
