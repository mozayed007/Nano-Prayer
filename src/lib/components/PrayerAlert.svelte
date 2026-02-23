<script lang="ts">
  import { fade, fly } from "svelte/transition";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";

  interface PrayerAlertPayload {
    prayer: string;
    alert_type: string;
    title: string;
    body: string;
  }

  let alert = $state<PrayerAlertPayload | null>(null);
  let visible = $state(false);

  onMount(() => {
    let unlisten: (() => void) | undefined;

    listen<PrayerAlertPayload>("prayer-alert", (event) => {
      alert = event.payload;
      visible = true;

      // Auto-dismiss after 60 seconds for non-prayer-time alerts
      if (alert.alert_type !== "on_time") {
        setTimeout(() => {
          visible = false;
        }, 60000);
      }
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
  });

  async function dismiss() {
    visible = false;
    try {
      await invoke("stop_audio");
    } catch (e) {
      console.error("Failed to stop audio on dismiss:", e);
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

      <div class="flex gap-3 relative z-10 w-full mt-2">
        <button
          onclick={dismiss}
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
