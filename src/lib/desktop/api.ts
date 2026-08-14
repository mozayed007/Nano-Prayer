export type UnlistenFn = () => void;

interface VisibilityEvent {
  visible: boolean;
}

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
        onVisibilityChanged?: (cb: (event: VisibilityEvent) => void) => UnlistenFn;
      };
    };
  }
}

function isTauriRuntime(): boolean {
  return (
    typeof window !== "undefined" &&
    ("__TAURI_INTERNALS__" in window || "__TAURI__" in window)
  );
}

function isElectronRuntime(): boolean {
  return typeof window !== "undefined" && typeof window.electronAPI !== "undefined";
}

export async function invoke<T = unknown>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (isTauriRuntime()) {
    const mod = await import("@tauri-apps/api/core");
    return mod.invoke<T>(command, args);
  }

  if (isElectronRuntime()) {
    return window.electronAPI!.invoke(command, args) as Promise<T>;
  }

  throw new Error(`Desktop runtime not available for invoke(${command})`);
}

export async function listen<T = unknown>(
  event: string,
  handler: (event: { payload: T }) => void,
): Promise<UnlistenFn> {
  if (isTauriRuntime()) {
    const mod = await import("@tauri-apps/api/event");
    return mod.listen<T>(event, handler as any);
  }

  if (isElectronRuntime()) {
    const unlisten = window.electronAPI!.listen(event, (payload) =>
      handler({ payload: payload as T }),
    );
    return unlisten;
  }

  return () => {};
}

interface DesktopWindow {
  hide: () => Promise<void>;
  show: () => Promise<void>;
  setFocus: () => Promise<void>;
  isVisible: () => Promise<boolean>;
  onVisibilityChanged: (cb: (event: VisibilityEvent) => void) => Promise<UnlistenFn>;
}

export function getCurrentWindow(): DesktopWindow {
  if (isElectronRuntime()) {
    return {
      hide: async () => window.electronAPI!.window.hide(),
      show: async () => window.electronAPI!.window.show(),
      setFocus: async () => window.electronAPI!.window.focus(),
      isVisible: async () => window.electronAPI!.window.isVisible(),
      onVisibilityChanged: async (cb: (event: VisibilityEvent) => void) => {
        if (window.electronAPI!.window.onVisibilityChanged) {
          return window.electronAPI!.window.onVisibilityChanged(cb);
        }
        return () => {};
      },
    };
  }

  if (isTauriRuntime()) {
    return {
      hide: async () => {
        await invoke("hide_main_window");
      },
      show: async () => {
        await invoke("show_main_window");
      },
      setFocus: async () =>
        (await import("@tauri-apps/api/window")).getCurrentWindow().setFocus(),
      isVisible: async () =>
        (await import("@tauri-apps/api/window")).getCurrentWindow().isVisible(),
      onVisibilityChanged: async (cb: (event: VisibilityEvent) => void): Promise<UnlistenFn> => {
        return listen<VisibilityEvent>("np-window-visibility", (event) => {
          cb(event.payload);
        });
      },
    };
  }

  return {
    hide: async () => {},
    show: async () => {},
    setFocus: async () => {},
    isVisible: async () => true,
    onVisibilityChanged: async () => () => {},
  };
}

export async function checkForUpdates() {
  if (isTauriRuntime()) {
    const mod = await import("@tauri-apps/plugin-updater");
    return mod.check();
  }

  if (isElectronRuntime()) {
    return invoke("desktop_check_update");
  }

  return null;
}

export async function isShortcutRegistered(shortcut: string): Promise<boolean> {
  if (isTauriRuntime()) {
    const mod = await import("@tauri-apps/plugin-global-shortcut");
    return mod.isRegistered(shortcut);
  }

  if (isElectronRuntime()) {
    return invoke<boolean>("desktop_is_shortcut_registered", { shortcut });
  }

  return false;
}

export async function registerShortcut(
  shortcut: string,
  cb?: (event: { state: string }) => void,
): Promise<void> {
  if (isTauriRuntime()) {
    const mod = await import("@tauri-apps/plugin-global-shortcut");
    return mod.register(shortcut, cb as any);
  }

  if (isElectronRuntime()) {
    await invoke("desktop_register_shortcut", { shortcut });
    return;
  }
}

export async function openDialog() {
  if (isTauriRuntime()) {
    const mod = await import("@tauri-apps/plugin-dialog");
    return mod.open({
      multiple: false,
      filters: [{ name: "Audio Files", extensions: ["mp3", "wav", "ogg"] }],
    });
  }

  if (isElectronRuntime()) {
    return invoke<string | null>("desktop_open_dialog");
  }

  return null;
}

export async function setAutostart(enabled: boolean): Promise<void> {
  if (isTauriRuntime()) {
    const mod = await import("@tauri-apps/plugin-autostart");
    if (enabled) {
      await mod.enable();
    } else {
      await mod.disable();
    }
    return;
  }

  if (isElectronRuntime()) {
    await invoke("desktop_set_autostart", { enabled });
  }
}

export async function getAppVersion(): Promise<string> {
  if (isTauriRuntime()) {
    const mod = await import("@tauri-apps/api/app");
    return mod.getVersion();
  }

  if (isElectronRuntime()) {
    return invoke<string>("desktop_get_version");
  }

  return "0.0.0";
}

export async function openExternal(url: string): Promise<void> {
  if (isTauriRuntime()) {
    const mod = await import("@tauri-apps/plugin-shell");
    await mod.open(url);
    return;
  }

  if (isElectronRuntime()) {
    await invoke("desktop_open_external", { url });
    return;
  }

  window.open(url, "_blank", "noopener,noreferrer");
}
