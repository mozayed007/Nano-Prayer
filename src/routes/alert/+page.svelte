<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core";
  import { getCurrentWindow } from "@tauri-apps/api/window";
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
  let audioPlaying = $state(false);

  async function startListening(retries = 3) {
    try {
      unlisten = await listen<PrayerAlertPayload>("prayer-alert", (event) => {
        activeAlert = event.payload;
        if (activeAlert.alert_type === "on_time") {
          audioPlaying = true;
        }
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
    // Determine if we are in Tauri
    const isTauri =
      typeof window !== "undefined" &&
      ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

    if (isTauri) {
      startListening();

      // Global shortcut or Escape to dismiss
      const handleKeydown = (e: KeyboardEvent) => {
        if (e.key === "Escape") dismiss();
      };
      window.addEventListener("keydown", handleKeydown);

      return () => {
        window.removeEventListener("keydown", handleKeydown);
        if (unlisten) unlisten();
      };
    }
  });

  onDestroy(() => {
    if (unlisten) unlisten();
  });

  async function dismiss() {
    activeAlert = null;
    audioPlaying = false;

    try {
      await invoke("stop_audio"); // Stop any playing sounds
      const appWindow = getCurrentWindow();
      await appWindow.hide();
    } catch (e) {
      console.error("Failed to dismiss alert window", e);
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

<!-- Borderless transparent canvas -->
<main
  class="w-full h-full bg-transparent p-3 flex items-center justify-center overflow-hidden"
  data-tauri-drag-region
>
  {#if activeAlert}
    <div
      class="relative w-full h-full {getBackgroundColor(
        activeAlert?.prayer,
      )} border border-white/20 rounded-2xl shadow-2xl p-5 flex flex-col justify-between overflow-hidden backdrop-blur-3xl text-white"
      in:fly={{ y: 20, duration: 600, easing: backOut }}
      out:fade={{ duration: 300 }}
      data-tauri-drag-region
    >
      <!-- Glass reflection -->
      <div
        class="absolute inset-0 bg-gradient-to-br from-white/10 to-transparent pointer-events-none rounded-2xl"
      ></div>

      <div class="z-10 flex gap-4 items-start relative pointer-events-none">
        <!-- Icon -->
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
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            class="text-blue-300"
          >
            <path d="M10.268 21a2 2 0 0 0 3.464 0" />
            <path
              d="M22 8c0-2.3-.8-4.3-2-6-1.5 2.5-3 3-5 3s-3.5-.5-5-3c-1.2 1.7-2 3.7-2 6 0 5-2 6-2 6h16s-2-1-2-6Z"
            />
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
          class="bg-white/10 hover:bg-white/20 text-white border border-white/20 hover:border-white/40 transition-all font-medium rounded-xl px-5 py-2 flex items-center gap-2 cursor-pointer shadow-sm active:scale-95"
          onclick={dismiss}
        >
          Dismiss
        </button>
      </div>
    </div>
  {:else}
    <div
      class="w-full h-full bg-slate-900/40 border border-white/10 rounded-2xl flex items-center justify-center text-white/40 backdrop-blur-md"
      data-tauri-drag-region
    >
      <div class="text-center font-medium animate-pulse">
        Waiting for alert...
      </div>
      <button
        class="absolute top-2 right-2 p-2 hover:bg-white/10 rounded-lg text-white/50 hover:text-white"
        onclick={dismiss}>✕</button
      >
    </div>
  {/if}
</main>
