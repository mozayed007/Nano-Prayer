import { contextBridge, ipcRenderer } from "electron";

type InvokePayload = {
  command: string;
  args?: Record<string, unknown>;
};

type EventPayload = {
  event: string;
  payload: unknown;
};

const listeners = new Map<string, Set<(payload: unknown) => void>>();

ipcRenderer.on("desktop:event", (_event: unknown, data: EventPayload) => {
  const set = listeners.get(data.event);
  if (!set) {
    return;
  }
  set.forEach((listener) => listener(data.payload));
});

const api = {
  invoke: (command: string, args?: Record<string, unknown>) =>
    ipcRenderer.invoke("desktop:invoke", { command, args } satisfies InvokePayload),
  listen: (event: string, cb: (payload: unknown) => void) => {
    if (!listeners.has(event)) {
      listeners.set(event, new Set());
    }
    listeners.get(event)!.add(cb);
    return () => {
      listeners.get(event)?.delete(cb);
    };
  },
  window: {
    hide: () => ipcRenderer.invoke("desktop:invoke", { command: "desktop_window_hide" }),
    show: () => ipcRenderer.invoke("desktop:invoke", { command: "desktop_window_show" }),
    focus: () => ipcRenderer.invoke("desktop:invoke", { command: "desktop_window_focus" }),
    isVisible: () => ipcRenderer.invoke("desktop:invoke", { command: "desktop_window_is_visible" }),
  },
};

contextBridge.exposeInMainWorld("electronAPI", api);
