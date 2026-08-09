<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { getAppVersion, openExternal, invoke } from "$lib/desktop/api";

  /** Native desktop update check result (Tauri + Electron). */
  type DesktopUpdateCheck = {
    available: boolean;
    currentVersion: string;
    latestVersion: string;
    releaseUrl: string;
    releaseNotes: string;
    publishedAt: string;
    isPrerelease: boolean;
    status: "update" | "current" | "ahead" | "empty" | "error" | string;
    message: string;
    error?: string | null;
  };

  let appVersion = $state("");
  let checking = $state(false);
  let updateMessage = $state("");
  let hasUpdate = $state(false);
  let releaseUrl = $state("");
  let lastStatus = $state<string>("");
  let checkError = $state(false);

  onMount(async () => {
    try {
      appVersion = await getAppVersion();
    } catch (e) {
      console.warn("Could not get app version", e);
      appVersion = "0.0.0";
    }
  });

  function isDesktop(): boolean {
    return (
      typeof window !== "undefined" &&
      ("__TAURI_INTERNALS__" in window ||
        "__TAURI__" in window ||
        "electronAPI" in window)
    );
  }

  async function checkForUpdates() {
    checking = true;
    updateMessage = "Checking GitHub for updates...";
    hasUpdate = false;
    releaseUrl = "";
    checkError = false;
    lastStatus = "";

    try {
      if (!isDesktop()) {
        updateMessage =
          "Update checks run in the desktop app. Open NanoPrayer (Tauri or Electron) to check.";
        checkError = true;
        return;
      }

      // Always go through native main process — WebView CSP blocks renderer fetch to api.github.com.
      const result = await invoke<DesktopUpdateCheck | null>("desktop_check_update");

      if (!result) {
        updateMessage =
          "Could not check for updates (no response). Try again later or open the releases page.";
        checkError = true;
        releaseUrl = "https://github.com/mozayed007/Nano-Prayer/releases";
        return;
      }

      lastStatus = result.status || "";
      hasUpdate = !!result.available;
      releaseUrl = result.releaseUrl || "";
      updateMessage =
        result.message ||
        (hasUpdate
          ? `Version ${result.latestVersion} is available!`
          : "You are on the latest version.");

      if (result.status === "error") {
        checkError = true;
        hasUpdate = false;
        if (!releaseUrl) {
          releaseUrl = "https://github.com/mozayed007/Nano-Prayer/releases";
        }
      }
    } catch (e) {
      checkError = true;
      hasUpdate = false;
      const msg = e instanceof Error ? e.message : String(e);
      // Surface actionable text instead of raw TypeError: Failed to fetch
      if (/failed to fetch|network|csp|connect/i.test(msg)) {
        updateMessage =
          "Could not reach GitHub from the app. Check your network, then try again.";
      } else {
        updateMessage = `Failed to check for updates: ${msg}`;
      }
      releaseUrl = "https://github.com/mozayed007/Nano-Prayer/releases";
    } finally {
      checking = false;
    }
  }

  async function openDownloadPage() {
    const url =
      releaseUrl || "https://github.com/mozayed007/Nano-Prayer/releases";
    try {
      updateMessage = "Opening browser...";
      await openExternal(url);
      // Restore context after open
      if (hasUpdate) {
        updateMessage = lastStatus === "update" ? updateMessage : "Opening browser...";
      }
    } catch (e) {
      updateMessage = `Could not open browser: ${e}`;
      checkError = true;
    }
  }

  function messageClass(): string {
    if (checkError) return "text-amber-400 font-medium";
    if (hasUpdate) return "text-green-500 font-medium";
    if (lastStatus === "ahead") return "text-blue-400 font-medium";
    return "text-[var(--text-muted)]";
  }
</script>

<svelte:head>
  <title>About - NanoPrayer</title>
</svelte:head>

<div
  class="about-page h-full min-h-0 text-[var(--text-main)] p-4 sm:p-6 md:p-8 flex flex-col items-center justify-center overflow-y-auto"
  in:fade={{ duration: 400 }}
>
  <div
    class="max-w-2xl w-full flex flex-col items-center text-center backdrop-blur-3xl bg-[var(--glass-bg)] border border-[var(--glass-border)] rounded-[2rem] p-8 md:p-12 shadow-[0_8px_32px_var(--glass-shadow)] relative overflow-hidden"
  >
    <!-- Decorative glow -->
    <div
      class="absolute left-1/2 top-0 w-full aspect-square max-w-[500px] bg-[radial-gradient(circle,rgba(255,255,255,0.08)_0%,transparent_70%)] rounded-full pointer-events-none -translate-x-1/2 -translate-y-1/2"
    ></div>

    <!-- Banner Image -->
    <div class="w-full max-w-[85%] md:max-w-[75%] mb-8 z-10 relative">
      <img
        src="/banner.png"
        alt="NanoPrayer Banner"
        class="w-full h-auto drop-shadow-xl select-none"
      />
    </div>

    <!-- Version Info -->
    <div class="z-10 mb-8 w-full">
      <h1 class="text-3xl font-bold tracking-tight mb-2">NanoPrayer</h1>
      <p
        class="text-[var(--text-muted)] text-sm uppercase tracking-widest font-mono"
      >
        Version {appVersion || "Unknown"}
      </p>
    </div>

    <!-- Update Section -->
    <div
      class="z-10 w-full bg-[var(--text-main)]/5 border border-[var(--glass-border)] rounded-2xl p-6 mb-8 flex flex-col items-center gap-4"
    >
      {#if checking}
        <div
          class="w-6 h-6 border-2 border-[var(--text-main)]/30 border-t-[var(--text-main)] rounded-full animate-spin"
          aria-hidden="true"
        ></div>
        <p class="text-sm text-[var(--text-muted)]" role="status">{updateMessage}</p>
      {:else}
        <p class="text-sm {messageClass()} min-h-[1.5rem]" role="status">
          {updateMessage || "Keep your app up to date"}
        </p>
        <div class="flex flex-wrap items-center justify-center gap-3">
          {#if hasUpdate}
            <button
              type="button"
              onclick={openDownloadPage}
              class="bg-green-600 hover:bg-green-500 text-white px-6 py-2.5 rounded-xl font-medium transition-all shadow-lg hover:shadow-green-500/25 active:scale-95 flex items-center gap-2"
            >
              <span>🌐</span> Go to Download Page
            </button>
            <button
              type="button"
              onclick={checkForUpdates}
              class="bg-[var(--text-main)]/10 hover:bg-[var(--text-main)]/20 text-[var(--text-main)] px-4 py-2.5 rounded-xl font-medium transition-all border border-[var(--glass-border)] active:scale-95 flex items-center gap-2 text-sm"
            >
              Re-check
            </button>
          {:else}
            <button
              type="button"
              onclick={checkForUpdates}
              class="bg-[var(--text-main)]/10 hover:bg-[var(--text-main)]/20 text-[var(--text-main)] px-6 py-2.5 rounded-xl font-medium transition-all border border-[var(--glass-border)] active:scale-95 flex items-center gap-2"
            >
              <span>🔄</span> Check for Updates
            </button>
            {#if checkError || lastStatus === "empty" || lastStatus === "error"}
              <button
                type="button"
                onclick={openDownloadPage}
                class="bg-[var(--text-main)]/10 hover:bg-[var(--text-main)]/20 text-[var(--text-main)] px-4 py-2.5 rounded-xl font-medium transition-all border border-[var(--glass-border)] active:scale-95 flex items-center gap-2 text-sm"
              >
                Open releases page
              </button>
            {/if}
          {/if}
        </div>
      {/if}
    </div>

    <!-- Credits -->
    <div class="z-10 w-full pt-6 border-t border-[var(--glass-border)]">
      <h3
        class="text-sm font-semibold uppercase tracking-wider text-[var(--text-muted)] mb-4"
      >
        Credits
      </h3>
      <p class="text-sm text-[var(--text-main)]/80">
        Banner and Icon Design by <a
          href="https://x.com/A7medV19"
          target="_blank"
          rel="noopener noreferrer"
          class="font-medium hover:text-blue-400 hover:underline transition-colors"
          >AhmedV19</a
        >
      </p>
    </div>
  </div>
</div>
