<script lang="ts">
  import { fade, fly } from "svelte/transition";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  interface PrayerAlertPayload {
    prayer: string;
    alert_type: string;
    title: string;
    body: string;
  }

  let alert = $state<PrayerAlertPayload | null>(null);
  let visible = $state(false);
  let dismissing = $state(false);

  onMount(() => {
    let unlistenAlert: UnlistenFn | undefined;
    let unlistenDismissed: UnlistenFn | undefined;

    void listen<PrayerAlertPayload>("prayer-alert", (event) => {
      alert = event.payload;
      visible = true;

      // Auto-dismiss after 60 seconds for non-prayer-time alerts
      if (alert.alert_type !== "on_time") {
        setTimeout(() => {
          visible = false;
        }, 60000);
      }
    })
      .then((fn) => {
        unlistenAlert = fn;
      })
      .catch((e) => {
        console.error("Failed to listen for prayer alerts:", e);
      });

    void listen("prayer-alert-dismissed", () => {
      visible = false;
      alert = null;
    })
      .then((fn) => {
        unlistenDismissed = fn;
      })
      .catch((e) => {
        console.error("Failed to listen for dismiss events:", e);
      });

    return () => {
      if (unlistenAlert) unlistenAlert();
      if (unlistenDismissed) unlistenDismissed();
    };
  });

  async function dismiss() {
    if (dismissing) return;
    dismissing = true;
    visible = false;
    alert = null;
    try {
      await invoke("dismiss_alert");
    } catch (e) {
      console.error("Failed to stop audio on dismiss:", e);
    } finally {
      dismissing = false;
    }
  }

  async function markPrayed() {
    if (!alert) return;
    if (dismissing) return;
    dismissing = true;
    try {
      await invoke("mark_prayer_completed", { prayer: alert.prayer });
    } catch (e) {
      console.error("Failed to mark prayer as completed:", e);
    } finally {
      dismissing = false;
    }
  }

  function getIcon(type: string) {
    switch (type) {
      case "before":
        return "⏳";
      case "on_time":
        return "🕋";
      case "after":
        return "🕰️";
      default:
        return "🔔";
    }
  }
</script>

{#if visible && alert}
  <div
    class="fixed bottom-6 right-6 sm:bottom-8 sm:right-8 z-[9999] max-w-sm w-full px-4 sm:px-0"
    in:fly={{ y: 50, duration: 400 }}
    out:fade={{ duration: 300 }}
  >
    <div
      class="glass-card bg-[var(--glass-bg)] border border-[var(--glass-border)] shadow-[0_8px_32px_var(--glass-shadow)] p-5 rounded-2xl flex flex-col gap-4 backdrop-blur-xl relative overflow-hidden text-[var(--text-main)]"
    >
      <!-- Glow effect based on alert type -->
      <div
        class="absolute -top-10 -right-10 w-32 h-32 bg-blue-500 blur-[60px] opacity-20 pointer-events-none"
      ></div>

      <div class="flex items-start gap-4 relative z-10">
        <div
          class="text-3xl bg-[var(--text-main)]/10 w-14 h-14 rounded-2xl shadow-inner border border-[var(--glass-border)] flex items-center justify-center shrink-0"
        >
          {getIcon(alert.alert_type)}
        </div>
        <div class="flex-1 pt-1 min-w-0">
          <h3
            class="font-bold text-lg leading-tight mb-1 truncate drop-shadow-sm"
          >
            {alert.title}
          </h3>
          <p class="text-sm text-[var(--text-muted)] leading-relaxed">
            {alert.body}
          </p>
        </div>
      </div>
      <button
        class="absolute top-3 right-3 z-20 p-2 hover:bg-[var(--text-main)]/10 rounded-lg text-[var(--text-muted)] hover:text-[var(--text-main)]"
        onclick={dismiss}
        aria-label="Dismiss alert"
      >
        ✕
      </button>

      <div class="flex gap-3 relative z-10 w-full mt-2">
        <button
          onclick={markPrayed}
          disabled={dismissing}
          class="flex-1 py-2.5 px-4 rounded-xl font-medium text-sm transition-all duration-300 bg-emerald-500/20 hover:bg-emerald-500/30 text-emerald-100 border border-emerald-400/40 hover:border-emerald-300/60 shadow-sm focus:outline-none active:scale-95"
        >
          I Prayed
        </button>
        <button
          onclick={dismiss}
          disabled={dismissing}
          class="flex-1 py-2.5 px-4 rounded-xl font-medium text-sm transition-all duration-300 bg-white/10 hover:bg-white/20 text-white border border-white/20 hover:border-white/40 shadow-sm focus:outline-none active:scale-95"
          style={alert.alert_type === "on_time"
            ? "background-color: rgba(239, 68, 68, 0.2); border-color: rgba(239, 68, 68, 0.4);"
            : ""}
        >
          {alert.alert_type === "on_time" ? "Stop & Dismiss" : "Dismiss"}
        </button>
      </div>
    </div>
  </div>
{/if}
