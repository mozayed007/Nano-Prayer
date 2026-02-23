<script lang="ts">
  import "../app.css";
  import { page } from "$app/stores";
  import { onMount } from "svelte";
  import { currentPrayer, clockFormat, theme } from "$lib/stores";
  import { fade } from "svelte/transition";
  import { invoke } from "@tauri-apps/api/core";
  import { check } from "@tauri-apps/plugin-updater";
  import { isRegistered, register } from "@tauri-apps/plugin-global-shortcut";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import PrayerAlert from "$lib/components/PrayerAlert.svelte";

  interface PrayerTimesResponse {
    next_prayer: string | null;
    // other fields we might need later
  }

  let { children } = $props();
  let globalError = $state<string | null>(null);

  async function loadConfig() {
    try {
      const cfg = await invoke<any>("get_config");
      if (cfg?.appearance?.clock_format)
        clockFormat.set(cfg.appearance.clock_format);
      if (cfg?.appearance?.theme) theme.set(cfg.appearance.theme);
    } catch (e) {
      console.error("Failed to load config:", e);
    }
  }

  async function updatePrayerTimes() {
    try {
      const times = await invoke<PrayerTimesResponse>("get_prayer_times");
      currentPrayer.set(times.next_prayer);
    } catch (e) {
      console.error("Failed to fetch prayer times:", e);
    }
  }

  onMount(() => {
    const handleError = (event: ErrorEvent) => {
      globalError = event.message + "\n" + (event.error?.stack || "");
    };
    const handleRejection = (event: PromiseRejectionEvent) => {
      globalError = "Unhandled Promise Rejection: " + String(event.reason);
    };

    window.addEventListener("error", handleError);
    window.addEventListener("unhandledrejection", handleRejection);

    // Initial fetch
    loadConfig();
    updatePrayerTimes();

    // Refresh every minute
    const interval = setInterval(updatePrayerTimes, 60000);

    // Check if in Tauri Context before setting up native plugins
    const isTauri =
      typeof window !== "undefined" &&
      ("__TAURI_INTERNALS__" in window || "__TAURI__" in window);

    if (isTauri) {
      // 1. Updater
      check()
        .then((update) => {
          if (update) {
            console.log(`NanoPrayer Update available: ${update.version}`);
            // Opting for silent install or background process can be done here.
            // For now, simple logging of the availability
          }
        })
        .catch((e) => console.error("Update check failed:", e));

      // 2. Global Shortcuts
      const shortcut = "CommandOrControl+Shift+P";
      isRegistered(shortcut)
        .then((registered) => {
          if (!registered) {
            register(shortcut, (event) => {
              if (event.state === "Released") {
                const win = getCurrentWindow();
                win.isVisible().then((visible) => {
                  if (visible) win.hide();
                  else {
                    win.show();
                    win.setFocus();
                  }
                });
              }
            }).catch((e) =>
              console.error("Global shortcut register failed", e),
            );
          }
        })
        .catch((e) => console.error(e));
    }

    return () => {
      window.removeEventListener("error", handleError);
      window.removeEventListener("unhandledrejection", handleRejection);
      clearInterval(interval);
    };
  });

  function getGradientClass(nextPrayer: string | null): string {
    switch (nextPrayer?.toLowerCase()) {
      case "fajr":
        return "bg-gradient-to-br from-[#0c1222] via-[#1a2744] to-[#1e3a5f]";
      case "sunrise":
        return "bg-gradient-to-br from-[#1a2744] via-[#2d4a6f] to-[#4a6fa5]";
      case "dhuhr":
        return "bg-gradient-to-br from-[#0f1729] via-[#1a2744] to-[#2d4a6f]";
      case "asr":
        return "bg-gradient-to-br from-[#0f1729] via-[#1e2d4d] to-[#2d4a6f]";
      case "maghrib":
        return "bg-gradient-to-br from-[#1a1a2e] via-[#2d3a4f] to-[#1e3a5f]";
      case "isha":
        return "bg-gradient-to-br from-[#080c14] via-[#0f1729] to-[#1a2744]";
      default:
        return "bg-gradient-to-br from-[#0f1729] via-[#1a2744] to-[#0c1222]";
    }
  }

  // Handle theme class on transition
  $effect(() => {
    if (typeof document !== "undefined") {
      if ($theme === "light") {
        document.body.classList.add("light");
      } else if ($theme === "dark") {
        document.body.classList.remove("light");
      } else {
        // System theme
        const isDark = window.matchMedia(
          "(prefers-color-scheme: dark)",
        ).matches;
        if (isDark) document.body.classList.remove("light");
        else document.body.classList.add("light");
      }
    }
  });
</script>

{#if globalError}
  <div
    class="fixed inset-0 z-[9999] bg-black/95 text-red-500 p-8 overflow-auto font-mono text-sm flex flex-col items-center justify-center"
  >
    <div
      class="glass-card bg-red-900/20 border border-red-500 p-6 rounded-2xl max-w-2xl w-full"
    >
      <h1 class="text-2xl font-bold mb-4 text-red-400">Application Error</h1>
      <pre
        class="whitespace-pre-wrap break-words bg-black/50 p-4 rounded-lg mb-4 text-xs md:text-sm">{globalError}</pre>
      <button
        onclick={() => window.location.reload()}
        class="bg-red-600 hover:bg-red-500 text-white px-6 py-2 rounded-lg font-medium transition shadow-lg shadow-red-900/20 w-full"
      >
        Reload App
      </button>
    </div>
  </div>
{:else if ($page.url.pathname as string) === "/alert"}
  <main
    class="w-screen h-screen overflow-hidden overflow-y-hidden text-[var(--text-main)] {$theme}"
  >
    {@render children()}
  </main>
{:else}
  <div
    class="h-dvh min-h-0 text-[var(--text-main)] transition-colors duration-1000 {$theme ===
    'light'
      ? 'bg-[#f8fafc]'
      : getGradientClass(
          $currentPrayer,
        )} flex flex-col relative overflow-hidden {$theme}"
  >
    <!-- Overlay for depth -->
    <div
      class="absolute inset-0 bg-[url('https://grainy-gradients.vercel.app/noise.svg')] opacity-20 pointer-events-none mix-blend-overlay"
    ></div>

    <div
      class="w-full flex justify-center absolute top-2 md:top-4 left-0 z-50 px-2 sm:px-4 pointer-events-none"
    >
      <nav
        class="app-nav pointer-events-auto backdrop-blur-xl bg-[var(--glass-bg)] border border-[var(--glass-border)] rounded-2xl px-2 py-2 flex justify-center items-center gap-1 shadow-[0_8px_32px_var(--glass-shadow)] max-w-full overflow-x-auto whitespace-nowrap"
      >
        <a
          href="/"
          aria-label="Praytime Dashboard"
          aria-current={$page.url.pathname === "/" ? "page" : undefined}
          class="flex items-center gap-2 px-3 md:px-4 lg:px-5 py-2 md:py-2.5 rounded-xl transition-all duration-300 hover:text-[var(--text-main)] hover:bg-[var(--text-main)]/10 focus-visible:ring-2 focus-visible:ring-white/50 focus-visible:outline-none relative group {$page
            .url.pathname === '/'
            ? 'text-[var(--text-main)] bg-[var(--text-main)]/20 shadow-md'
            : 'text-[var(--text-muted)]'}"
        >
          <span class="text-xl group-hover:scale-110 transition-transform"
            >🕒</span
          >
          <span class="hidden lg:inline text-sm font-semibold tracking-wide"
            >Praytime</span
          >
          {#if $page.url.pathname === "/"}
            <div
              class="absolute -bottom-1 left-1/2 -translate-x-1/2 w-8 h-1 bg-[var(--text-main)] rounded-t-full"
              in:fade
            ></div>
          {/if}
        </a>
        <a
          href="/schedule"
          aria-label="Monthly Schedule"
          aria-current={$page.url.pathname.startsWith("/schedule")
            ? "page"
            : undefined}
          class="flex items-center gap-2 px-3 md:px-4 lg:px-5 py-2 md:py-2.5 rounded-xl transition-all duration-300 hover:text-[var(--text-main)] hover:bg-[var(--text-main)]/10 focus-visible:ring-2 focus-visible:ring-white/50 focus-visible:outline-none relative group {$page.url.pathname.startsWith(
            '/schedule',
          )
            ? 'text-[var(--text-main)] bg-[var(--text-main)]/20 shadow-md'
            : 'text-[var(--text-muted)]'}"
        >
          <span class="text-xl group-hover:scale-110 transition-transform"
            >📅</span
          >
          <span class="hidden lg:inline text-sm font-semibold tracking-wide"
            >Schedule</span
          >
          {#if $page.url.pathname.startsWith("/schedule")}
            <div
              class="absolute -bottom-1 left-1/2 -translate-x-1/2 w-8 h-1 bg-[var(--text-main)] rounded-t-full"
              in:fade
            ></div>
          {/if}
        </a>
        <a
          href="/statistics"
          aria-label="Prayer Statistics"
          aria-current={$page.url.pathname.startsWith("/statistics")
            ? "page"
            : undefined}
          class="flex items-center gap-2 px-3 md:px-4 lg:px-5 py-2 md:py-2.5 rounded-xl transition-all duration-300 hover:text-[var(--text-main)] hover:bg-[var(--text-main)]/10 focus-visible:ring-2 focus-visible:ring-white/50 focus-visible:outline-none relative group {$page.url.pathname.startsWith(
            '/statistics',
          )
            ? 'text-[var(--text-main)] bg-[var(--text-main)]/20 shadow-md'
            : 'text-[var(--text-muted)]'}"
        >
          <span class="text-xl group-hover:scale-110 transition-transform"
            >📊</span
          >
          <span class="hidden lg:inline text-sm font-semibold tracking-wide"
            >Stats</span
          >
          {#if $page.url.pathname.startsWith("/statistics")}
            <div
              class="absolute -bottom-1 left-1/2 -translate-x-1/2 w-8 h-1 bg-[var(--text-main)] rounded-t-full"
              in:fade
            ></div>
          {/if}
        </a>
        <a
          href="/settings"
          aria-label="Application Settings"
          aria-current={$page.url.pathname.startsWith("/settings")
            ? "page"
            : undefined}
          class="flex items-center gap-2 px-3 md:px-4 lg:px-5 py-2 md:py-2.5 rounded-xl transition-all duration-300 hover:text-[var(--text-main)] hover:bg-[var(--text-main)]/10 focus-visible:ring-2 focus-visible:ring-white/50 focus-visible:outline-none relative group {$page.url.pathname.startsWith(
            '/settings',
          )
            ? 'text-[var(--text-main)] bg-[var(--text-main)]/20 shadow-md'
            : 'text-[var(--text-muted)]'}"
        >
          <span class="text-xl group-hover:scale-110 transition-transform"
            >⚙️</span
          >
          <span class="hidden lg:inline text-sm font-semibold tracking-wide"
            >Settings</span
          >
          {#if $page.url.pathname.startsWith("/settings")}
            <div
              class="absolute -bottom-1 left-1/2 -translate-x-1/2 w-8 h-1 bg-[var(--text-main)] rounded-t-full"
              in:fade
            ></div>
          {/if}
        </a>
        <a
          href="/qibla"
          aria-label="Qibla Compass"
          aria-current={$page.url.pathname.startsWith("/qibla")
            ? "page"
            : undefined}
          class="flex items-center gap-2 px-3 md:px-4 lg:px-5 py-2 md:py-2.5 rounded-xl transition-all duration-300 hover:text-[var(--text-main)] hover:bg-[var(--text-main)]/10 focus-visible:ring-2 focus-visible:ring-white/50 focus-visible:outline-none relative group {$page.url.pathname.startsWith(
            '/qibla',
          )
            ? 'text-[var(--text-main)] bg-[var(--text-main)]/20 shadow-md'
            : 'text-[var(--text-muted)]'}"
        >
          <span class="text-xl group-hover:scale-110 transition-transform"
            >🧭</span
          >
          <span class="hidden lg:inline text-sm font-semibold tracking-wide"
            >Qibla</span
          >
          {#if $page.url.pathname.startsWith("/qibla")}
            <div
              class="absolute -bottom-1 left-1/2 -translate-x-1/2 w-8 h-1 bg-[var(--text-main)] rounded-t-full"
              in:fade
            ></div>
          {/if}
        </a>
        <a
          href="/about"
          aria-label="About the App"
          aria-current={$page.url.pathname.startsWith("/about")
            ? "page"
            : undefined}
          class="flex items-center gap-2 px-3 md:px-4 lg:px-5 py-2 md:py-2.5 rounded-xl transition-all duration-300 hover:text-[var(--text-main)] hover:bg-[var(--text-main)]/10 focus-visible:ring-2 focus-visible:ring-white/50 focus-visible:outline-none relative group {$page.url.pathname.startsWith(
            '/about',
          )
            ? 'text-[var(--text-main)] bg-[var(--text-main)]/20 shadow-md'
            : 'text-[var(--text-muted)]'}"
        >
          <span class="text-xl group-hover:scale-110 transition-transform"
            >ℹ️</span
          >
          <span class="hidden lg:inline text-sm font-semibold tracking-wide"
            >About</span
          >
          {#if $page.url.pathname.startsWith("/about")}
            <div
              class="absolute -bottom-1 left-1/2 -translate-x-1/2 w-8 h-1 bg-[var(--text-main)] rounded-t-full"
              in:fade
            ></div>
          {/if}
        </a>
      </nav>
    </div>

    <main
      class="flex-1 min-h-0 w-full mx-auto relative z-10 pt-[4.5rem] md:pt-20 pb-3 px-2 sm:px-4"
    >
      {@render children()}
    </main>

    <PrayerAlert />
  </div>
{/if}

<style>
  .app-nav {
    scrollbar-width: none;
  }
  .app-nav::-webkit-scrollbar {
    display: none;
  }

  /* Global Scrollbar Styling */
  :global(::-webkit-scrollbar) {
    width: 8px;
    height: 8px;
  }
  :global(::-webkit-scrollbar-track) {
    background: rgba(0, 0, 0, 0.1);
  }
  :global(::-webkit-scrollbar-thumb) {
    background: rgba(255, 255, 255, 0.2);
    border-radius: 4px;
  }
  :global(::-webkit-scrollbar-thumb:hover) {
    background: rgba(255, 255, 255, 0.3);
  }
</style>
