<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import {
    listen,
    invoke,
    getCurrentWindow,
    type UnlistenFn,
  } from "$lib/desktop/api";
  import { fade, fly } from "svelte/transition";
  import { backOut } from "svelte/easing";

  interface PrayerAlertPayload {
    prayer: string;
    alert_type: string;
    title: string;
    body: string;
  }

  let activeAlert = $state<PrayerAlertPayload | null>(null);
  let unlisten: UnlistenFn | null = null;
  let unlistenDismissed: UnlistenFn | null = null;
  let audioPlaying = $state(false);
  let dismissing = $state(false);

  async function startListening(retries = 3) {
    try {
      unlisten = await listen<PrayerAlertPayload>("prayer-alert", (event) => {
        activeAlert = event.payload;
        audioPlaying = activeAlert.alert_type === "on_time";
      });
    } catch (e) {
      console.warn(`Failed to start event listener (attempts left: ${retries - 1}):`, e);
      if (retries > 1) {
        await new Promise((r) => setTimeout(r, 500));
        return startListening(retries - 1);
      }
      console.error("Could not start prayer-alert listener after retries", e);
    }
  }

  onMount(() => {
    // Determine if we are in desktop runtime
    const isDesktop =
      typeof window !== "undefined" &&
      ("__TAURI_INTERNALS__" in window ||
        "__TAURI__" in window ||
        "electronAPI" in window);

    if (isDesktop) {
      void startListening();
      void invoke<PrayerAlertPayload | null>("get_active_alert")
        .then((payload) => {
          if (!payload) return;
          activeAlert = payload;
          audioPlaying = payload.alert_type === "on_time";
        })
        .catch((e) => {
          console.warn("Failed to hydrate active alert payload", e);
        });
      void listen("prayer-alert-dismissed", async () => {
        activeAlert = null;
        audioPlaying = false;
        const appWindow = getCurrentWindow();
        try {
          await appWindow.hide();
        } catch (e) {
          console.warn("Failed to hide alert window on dismiss event", e);
        }
      })
        .then((fn) => {
          unlistenDismissed = fn;
        })
        .catch((e) => {
          console.error("Failed to listen for dismiss events", e);
        });

      // Global shortcut or Escape to dismiss
      const handleKeydown = (e: KeyboardEvent) => {
        if (e.key === "Escape") dismiss();
      };
      window.addEventListener("keydown", handleKeydown);

      return () => {
        window.removeEventListener("keydown", handleKeydown);
        if (unlisten) unlisten();
        if (unlistenDismissed) unlistenDismissed();
      };
    }
  });

  onDestroy(() => {
    if (unlisten) unlisten();
    if (unlistenDismissed) unlistenDismissed();
  });

  async function dismiss() {
    if (dismissing) return;
    dismissing = true;
    activeAlert = null;
    audioPlaying = false;

    try {
      await invoke("dismiss_alert");
    } catch (e) {
      console.error("Failed to dismiss alert window", e);
    } finally {
      try {
        await getCurrentWindow().hide();
      } catch (e) {
        console.warn("Failed to hide alert window after dismiss", e);
      }
      dismissing = false;
    }
  }

  async function markPrayed() {
    if (!activeAlert) return;
    if (dismissing) return;
    dismissing = true;
    try {
      await invoke("mark_prayer_completed", { prayer: activeAlert.prayer });
    } catch (e) {
      console.error("Failed to mark prayer as completed", e);
    } finally {
      activeAlert = null;
      audioPlaying = false;
      try {
        await getCurrentWindow().hide();
      } catch (e) {
        console.warn("Failed to hide alert window after prayer completion", e);
      }
      dismissing = false;
    }
  }

  // Get background color based on prayer
  function getBackgroundColor(prayerName: string | undefined): string {
    if (!prayerName) return "bg-slate-900";
    const p = prayerName.toLowerCase();
    switch (p) {
      case "fajr":
        return "bg-[#0c1222]";
      case "sunrise":
        return "bg-[#2d4a6f]";
      case "dhuhr":
        return "bg-[#1a2744]";
      case "asr":
        return "bg-[#1e2d4d]";
      case "maghrib":
        return "bg-[#1e3a5f]";
      case "isha":
        return "bg-[#080c14]";
      default:
        return "bg-slate-900";
    }
  }
</script>

<svelte:head>
  <title>Prayer Alert</title>
</svelte:head>

<main
  class="w-full h-full bg-slate-950 p-3 flex items-center justify-center overflow-hidden relative"
>
  <button
    class="absolute top-2 right-2 z-30 p-2 hover:bg-white/10 rounded-lg text-white/70 hover:text-white"
    onclick={dismiss}
    aria-label="Close alert overlay"
  >
    ✕
  </button>
  {#if activeAlert}
    <div
      class="relative w-full h-full {getBackgroundColor(
        activeAlert?.prayer,
      )} border border-white/20 rounded-2xl shadow-2xl p-5 flex flex-col justify-between overflow-hidden text-white"
      in:fly={{ y: 20, duration: 600, easing: backOut }}
      out:fade={{ duration: 300 }}
    >
      <!-- Glass reflection -->
      <div
        class="absolute inset-0 bg-gradient-to-br from-white/10 to-transparent pointer-events-none rounded-2xl"
      ></div>

      <div class="z-10 flex gap-4 items-start relative pointer-events-none">
        <!-- Icon: Kaaba/Mosque -->
        <div
          class="bg-blue-500/20 p-3 rounded-full border border-blue-400/30 flex-shrink-0 flex items-center justify-center"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="text-blue-300"
          >
            <!-- Kaaba cube body -->
            <rect x="4" y="11" width="16" height="10" rx="0.5" />
            <!-- Kiswah band (decorative stripe) -->
            <line x1="4" y1="15" x2="20" y2="15" />
            <!-- Arch / dome above Kaaba -->
            <path d="M8 11 Q12 4 16 11" />
            <!-- Ground line -->
            <line x1="2" y1="21" x2="22" y2="21" />
          </svg>
        </div>

        <!-- Text content -->
        <div class="flex-1 min-w-0">
          <h2
            class="text-lg font-bold truncate drop-shadow-sm leading-tight text-white mb-1"
          >
            {activeAlert.title}
          </h2>
          <p class="text-sm text-blue-100/80 drop-shadow-sm truncate">
            {activeAlert.body}
          </p>
        </div>
      </div>
      <!-- Action -->
      <div class="z-10 flex justify-end items-center mt-3 gap-3">
        {#if audioPlaying}
          <div
            class="flex items-center gap-1.5 px-2 text-blue-300/60 pointer-events-none mr-auto"
          >
            <div
              class="w-1 h-3 bg-current rounded-full animate-[pulse_1s_ease-in-out_infinite]"
            ></div>
            <div
              class="w-1 h-4 bg-current rounded-full animate-[pulse_1s_ease-in-out_infinite_0.2s]"
            ></div>
            <div
              class="w-1 h-2 bg-current rounded-full animate-[pulse_1s_ease-in-out_infinite_0.4s]"
            ></div>
          </div>
        {/if}
        <button
          class="bg-emerald-500/20 hover:bg-emerald-500/30 text-emerald-100 border border-emerald-400/40 hover:border-emerald-300/60 transition-all font-medium rounded-xl px-5 py-2 flex items-center gap-2 cursor-pointer shadow-sm active:scale-95"
          onclick={markPrayed}
          disabled={dismissing}
        >
          I Prayed
        </button>
        <button
          class="bg-white/10 hover:bg-white/20 text-white border border-white/20 hover:border-white/40 transition-all font-medium rounded-xl px-5 py-2 flex items-center gap-2 cursor-pointer shadow-sm active:scale-95"
          onclick={dismiss}
          disabled={dismissing}
        >
          Dismiss
        </button>
      </div>
    </div>
  {:else}
    <div
      class="w-full h-full bg-slate-900 border border-white/10 rounded-2xl flex items-center justify-center text-white/40"
    >
      <div class="text-center font-medium">
        Waiting for alert...
      </div>
    </div>
  {/if}
</main>
