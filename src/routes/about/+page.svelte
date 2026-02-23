<script lang="ts">
  import { onMount } from "svelte";
  import { fade } from "svelte/transition";
  import { getVersion } from "@tauri-apps/api/app";
  import { open } from "@tauri-apps/plugin-shell";

  let appVersion = $state("");
  let checking = $state(false);
  let updateMessage = $state("");
  let hasUpdate = $state(false);
  let releaseUrl = $state("");

  onMount(async () => {
    try {
      appVersion = await getVersion();
    } catch (e) {
      console.warn("Could not get app version", e);
    }
  });

  async function checkForUpdates() {
    checking = true;
    updateMessage = "Checking GitHub for updates...";
    hasUpdate = false;
    releaseUrl = "";

    try {
      // Fetch all releases (including pre-releases) from GitHub API
      const response = await fetch(
        "https://api.github.com/repos/mozayed007/Nano-Prayer/releases",
      );

      if (response.status === 404) {
        updateMessage = "No releases found yet.";
        return;
      }

      if (!response.ok) {
        throw new Error(`GitHub API returned ${response.status}`);
      }

      const data = await response.json();

      if (!Array.isArray(data) || data.length === 0) {
        updateMessage = "No releases found yet.";
        return;
      }

      // GitHub returns ordered by creation date, [0] is the newest release/pre-release
      const latestRelease = data[0];
      const latestVersion = latestRelease.tag_name.replace(/^v/, ""); // Remove 'v' prefix if present

      // Simple version comparison
      if (appVersion && isNewerVersion(appVersion, latestVersion)) {
        hasUpdate = true;
        const releaseType = latestRelease.prerelease ? " (Pre-release)" : "";
        updateMessage = `Version ${latestVersion}${releaseType} is available!`;
        releaseUrl = latestRelease.html_url;
      } else {
        updateMessage = "You are on the latest version.";
      }
    } catch (e) {
      updateMessage = `Failed to check for updates: ${e}`;
    } finally {
      checking = false;
    }
  }

  // Helper to compare semver strings (very basic)
  function isNewerVersion(current: string, latest: string): boolean {
    const v1 = current.split(".").map(Number);
    const v2 = latest.split(".").map(Number);

    for (let i = 0; i < Math.max(v1.length, v2.length); i++) {
      const num1 = v1[i] || 0;
      const num2 = v2[i] || 0;
      if (num2 > num1) return true;
      if (num2 < num1) return false;
    }
    return false;
  }

  async function openDownloadPage() {
    if (releaseUrl) {
      updateMessage = "Opening browser...";
      await open(releaseUrl);
    }
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
        ></div>
        <p class="text-sm text-[var(--text-muted)]">{updateMessage}</p>
      {:else}
        <p
          class="text-sm {hasUpdate
            ? 'text-green-500 font-medium'
            : 'text-[var(--text-muted)]'} min-h-[1.5rem]"
        >
          {updateMessage || "Keep your app up to date"}
        </p>
        {#if hasUpdate}
          <button
            onclick={openDownloadPage}
            class="bg-green-600 hover:bg-green-500 text-white px-6 py-2.5 rounded-xl font-medium transition-all shadow-lg hover:shadow-green-500/25 active:scale-95 flex items-center gap-2"
          >
            <span>🌐</span> Go to Download Page
          </button>
        {:else}
          <button
            onclick={checkForUpdates}
            class="bg-[var(--text-main)]/10 hover:bg-[var(--text-main)]/20 text-[var(--text-main)] px-6 py-2.5 rounded-xl font-medium transition-all border border-[var(--glass-border)] active:scale-95 flex items-center gap-2"
          >
            <span>🔄</span> Check for Updates
          </button>
        {/if}
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
