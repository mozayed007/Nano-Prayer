"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const electron_1 = require("electron");
const listeners = new Map();
electron_1.ipcRenderer.on("desktop:event", (_event, data) => {
    const set = listeners.get(data.event);
    if (!set) {
        return;
    }
    set.forEach((listener) => listener(data.payload));
});
const api = {
    invoke: (command, args) => electron_1.ipcRenderer.invoke("desktop:invoke", { command, args }),
    listen: (event, cb) => {
        if (!listeners.has(event)) {
            listeners.set(event, new Set());
        }
        listeners.get(event).add(cb);
        return () => {
            listeners.get(event)?.delete(cb);
        };
    },
    window: {
        hide: () => electron_1.ipcRenderer.invoke("desktop:invoke", { command: "desktop_window_hide" }),
        show: () => electron_1.ipcRenderer.invoke("desktop:invoke", { command: "desktop_window_show" }),
        focus: () => electron_1.ipcRenderer.invoke("desktop:invoke", { command: "desktop_window_focus" }),
        isVisible: () => electron_1.ipcRenderer.invoke("desktop:invoke", { command: "desktop_window_is_visible" }),
    },
};
electron_1.contextBridge.exposeInMainWorld("electronAPI", api);
