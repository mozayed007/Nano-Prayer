<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { invoke, openDialog, setAutostart } from "$lib/desktop/api";
  import { fade } from "svelte/transition";
  import { clockFormat, theme, visualMode } from "$lib/stores";
  import type { AppConfig, SavedLocation, VisualMode } from "$lib/types";

  let config = $state<AppConfig | null>(null);
  let activeTab = $state("location");
  let loading = $state(true);
  let saveStatus = $state("");

  // Location search state
  let searchQuery = $state("");
  let searchResults = $state<any[]>([]);
  let searching = $state(false);

  const tabs = [
    { id: "location", label: "Location" },
    { id: "calculation", label: "Calculation" },
    { id: "reminders", label: "Reminders" },
    { id: "appearance", label: "Appearance" },
    { id: "audio", label: "Audio" },
    { id: "advanced", label: "Advanced" },
  ];

  const calculationMethods = [
    "MuslimWorldLeague",
    "Egyptian",
    "Karachi",
    "UmmAlQura",
    "Dubai",
    "MoonsightingCommittee",
    "NorthAmerica",
    "Kuwait",
    "Qatar",
    "Singapore",
    "Tehran",
    "Turkey",
  ];

  const asrMadhabs = ["Shafi", "Hanafi"];
  const highLatitudeRules = [
    "MiddleOfTheNight",
    "SeventhOfTheNight",
    "TwilightAngle",
  ];
  const prayerOrder = ["fajr", "sunrise", "dhuhr", "asr", "maghrib", "isha"];

  function createDefaultReminder() {
    return {
      enabled: true,
      before_enabled: true,
      minutes_before: 15,
      play_sound_before: false,
      play_adhan: true,
      after_enabled: true,
      minutes_after: 0,
      play_sound_after: false,
      custom_sound: null,
      custom_reminder_sound: null,
      volume: 0.7,
      show_notification: true,
    };
  }

  async function loadConfig() {
    // Check if running in desktop environment
    if (
      typeof window !== "undefined" &&
      !("__TAURI_INTERNALS__" in window) &&
      !("electronAPI" in window)
    ) {
      console.error("Desktop API not available.");
      loading = false;
      return;
    }
    try {
      config = await invoke<AppConfig>("get_config");
      if (config) {
        for (const prayer of prayerOrder) {
          if (!(prayer in config.reminders)) {
            config.reminders[prayer] = createDefaultReminder();
          }
        }
        if (!config.appearance.visual_mode) {
          config.appearance.visual_mode = "performance";
        }
        config.appearance.animations_enabled =
          config.appearance.visual_mode === "glass";
        for (const settings of Object.values(config.reminders)) {
          if (settings.before_enabled === undefined) {
            settings.before_enabled = true;
          }
          if (settings.after_enabled === undefined) {
            settings.after_enabled = true;
          }
          if (settings.minutes_before < 0) {
            settings.minutes_before = 0;
          }
          if (settings.minutes_after < 0) {
            settings.minutes_after = 0;
          }
          // Ensure new field
          if (settings.custom_reminder_sound === undefined) {
            settings.custom_reminder_sound = null;
          }
        }
      }
    } catch (e) {
      console.error(e);
    } finally {
      loading = false;
    }
  }

  async function saveConfig() {
    if (!config) return;
    try {
      await invoke("save_config", { config });

      // Handle autostart
      if (config.advanced.auto_start) {
        await setAutostart(true).catch((e) =>
          console.error("Failed to enable autostart:", e),
        );
      } else {
        await setAutostart(false).catch((e) =>
          console.error("Failed to disable autostart:", e),
        );
      }

      // Sync stores
      $clockFormat = config.appearance.clock_format;
      $theme = config.appearance.theme;
      const mode: VisualMode =
        config.appearance.visual_mode === "glass" ? "glass" : "performance";
      config.appearance.visual_mode = mode;
      config.appearance.animations_enabled = mode === "glass";
      visualMode.set(mode);
      saveStatus = "Saved successfully!";
      setTimeout(() => (saveStatus = ""), 3000);
    } catch (e) {
      saveStatus = "Error saving: " + e;
    }
  }

  async function searchCity() {
    if (searchQuery.length < 3) return;
    searching = true;
    try {
      searchResults = await invoke("search_cities", { query: searchQuery });
    } catch (e) {
      console.error(e);
    } finally {
      searching = false;
    }
  }

  function addLocation(city: any) {
    if (!config) return;
    const newLoc: SavedLocation = {
      id: `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`,
      name: `${city.name}, ${city.country}`,
      coordinates: { latitude: city.latitude, longitude: city.longitude },
      timezone: city.timezone,
      is_favorite: false,
      is_default: config.locations.length === 0,
      calculation_method: null,
      notes: null,
      hijri_offset: null,
      hijri_auto_align: false,
    };
    config.locations = [...config.locations, newLoc];
    config.current_location_index = config.locations.length - 1;
    searchResults = [];
    searchQuery = "";
  }

  let alignBusy = $state(false);
  let alignStatus = $state("");

  async function autoAlignHijriForCurrentCity() {
    if (!config) return;
    alignBusy = true;
    alignStatus = "";
    try {
      // Prefer online authority (Aladhan) via desktop command; Electron fetches if fields omitted.
      const result = await invoke<{
        offset: number;
        observed: string;
        calculated: string;
        location_name: string;
        source: string;
      }>("align_hijri_for_location", {
        locationIndex: config.current_location_index,
        autoAlign: true,
      });
      // Reload config so location.hijri_offset is reflected
      config = await invoke<AppConfig>("get_config");
      alignStatus = `Aligned ${result.location_name}: offset ${result.offset >= 0 ? "+" : ""}${result.offset} (observed ${result.observed}, source ${result.source})`;
    } catch (e) {
      // Tauri requires observed fields – fetch Aladhan in UI then pass through
      try {
        const now = new Date();
        const dd = String(now.getDate()).padStart(2, "0");
        const mm = String(now.getMonth() + 1).padStart(2, "0");
        const yyyy = now.getFullYear();
        const response = await fetch(
          `https://api.aladhan.com/v1/gToH/${dd}-${mm}-${yyyy}`,
        );
        if (!response.ok) throw new Error(`Aladhan HTTP ${response.status}`);
        const data = await response.json();
        const h = data.data.hijri;
        const result = await invoke<{
          offset: number;
          observed: string;
          location_name: string;
        }>("align_hijri_for_location", {
          locationIndex: config.current_location_index,
          observedYear: parseInt(h.year, 10),
          observedMonth: h.month.number,
          observedDay: parseInt(h.day, 10),
          autoAlign: true,
        });
        config = await invoke<AppConfig>("get_config");
        alignStatus = `Aligned ${result.location_name}: offset ${result.offset >= 0 ? "+" : ""}${result.offset} (observed ${result.observed})`;
      } catch (e2) {
        alignStatus = `Align failed: ${String(e2)}`;
      }
    } finally {
      alignBusy = false;
    }
  }

  function removeLocation(index: number) {
    if (!config) return;
    config.locations = config.locations.filter((_, i) => i !== index);
    if (config.current_location_index >= config.locations.length) {
      config.current_location_index = Math.max(0, config.locations.length - 1);
    }
  }

  async function selectAdhanFile(prayer: string) {
    if (!config) return;
    try {
      const selected = await openDialog();

      if (selected && typeof selected === "string") {
        if (config.reminders[prayer]) {
          config.reminders[prayer].custom_sound = selected;
        }
      }
    } catch (e) {
      console.error(e);
    }
  }

  function clearAdhanFile(prayer: string) {
    if (config && config.reminders[prayer]) {
      config.reminders[prayer].custom_sound = null;
      if (previewingPrayer === prayer) {
        void stopPreview();
      }
    }
  }

  async function selectReminderFile(prayer: string) {
    if (!config) return;
    try {
      const selected = await openDialog();
      if (selected && typeof selected === "string") {
        if (config.reminders[prayer]) {
          config.reminders[prayer].custom_reminder_sound = selected;
        }
      }
    } catch (e) {
      console.error(e);
    }
  }

  function clearReminderFile(prayer: string) {
    if (config && config.reminders[prayer]) {
      config.reminders[prayer].custom_reminder_sound = null;
      if (previewingReminderPrayer === prayer) {
        void stopReminderPreview();
      }
    }
  }

  let previewingPrayer = $state<string | null>(null);
  let previewPaused = $state(false);
  let previewBusy = $state(false);
  let previewError = $state("");

  function getPreviewActionLabel(prayer: string): string {
    if (previewingPrayer !== prayer) return "Preview";
    return previewPaused ? "Resume" : "Pause";
  }

  function isPreviewing(prayer: string): boolean {
    return previewingPrayer === prayer;
  }

  async function togglePreview(prayer: string) {
    if (!config) return;
    if (previewBusy) return;
    previewBusy = true;
    previewError = "";

    try {
      if (previewingPrayer === prayer) {
        if (previewPaused) {
          await invoke("resume_audio");
          previewPaused = false;
        } else {
          await invoke("pause_audio");
          previewPaused = true;
        }
        return;
      }

      // Stop any currently playing preview before switching prayers.
      await invoke("stop_audio").catch(() => {});

      const settings = config.reminders[prayer];
      await invoke("play_adhan", {
        customPath: settings?.custom_sound ?? null,
        volume: config.audio.global_volume,
        isFajr: prayer === "fajr",
      });

      previewingPrayer = prayer;
      previewPaused = false;
    } catch (e) {
      previewError = `Audio preview failed: ${String(e)}`;
      previewingPrayer = null;
      previewPaused = false;
    } finally {
      previewBusy = false;
    }
  }

  async function stopPreview() {
    if (previewBusy) return;
    previewBusy = true;

    try {
      await invoke("stop_audio");
    } catch (e) {
      previewError = `Failed to stop preview: ${String(e)}`;
    } finally {
      previewingPrayer = null;
      previewPaused = false;
      previewBusy = false;
    }
  }

  $effect(() => {
    if (!config || !previewingPrayer) return;
    const currentSettings = config.reminders[previewingPrayer];
    if (!currentSettings?.play_adhan) {
      void stopPreview();
    }
  });

  let previewingReminderPrayer = $state<string | null>(null);
  let reminderPreviewBusy = $state(false);
  let reminderPreviewError = $state("");

  function isPreviewingReminder(prayer: string): boolean {
    return previewingReminderPrayer === prayer;
  }

  async function toggleReminderPreview(prayer: string) {
    if (!config) return;
    if (reminderPreviewBusy) return;
    reminderPreviewBusy = true;
    reminderPreviewError = "";

    try {
      if (previewingReminderPrayer === prayer) {
        await invoke("stop_audio").catch(() => {});
        previewingReminderPrayer = null;
        return;
      }

      // Stop any currently playing preview before switching.
      await invoke("stop_audio").catch(() => {});

      const settings = config.reminders[prayer];
      await invoke("play_reminder_sound", {
        customPath: settings?.custom_reminder_sound ?? null,
        volume: config.audio.global_volume,
      });

      previewingReminderPrayer = prayer;
    } catch (e) {
      reminderPreviewError = `Reminder preview failed: ${String(e)}`;
      previewingReminderPrayer = null;
    } finally {
      reminderPreviewBusy = false;
    }
  }

  async function stopReminderPreview() {
    if (reminderPreviewBusy) return;
    reminderPreviewBusy = true;
    try {
      await invoke("stop_audio");
    } catch (e) {
      reminderPreviewError = `Failed to stop: ${String(e)}`;
    } finally {
      previewingReminderPrayer = null;
      reminderPreviewBusy = false;
    }
  }

  onMount(loadConfig);
  onDestroy(() => {
    invoke("stop_audio").catch(() => {});
  });
</script>

<svelte:head>
  <title>Settings - NanoPrayer</title>
</svelte:head>

<div
  class="settings-shell h-full min-h-0 bg-transparent text-[var(--text-main)] p-2 sm:p-4 md:p-6 flex flex-col pt-1 md:pt-2 overflow-hidden"
  in:fade={{ duration: 400 }}
>
  <div class="settings-frame mx-auto w-full h-full flex flex-col overflow-hidden">
    <header
      class="flex flex-col md:flex-row md:items-center justify-between gap-2 md:gap-4 mb-3 md:mb-6 relative flex-shrink-0"
    >
      <!-- Subtle glow under title -->
      <div
        class="absolute left-0 top-0 w-[150%] aspect-square max-w-[300px] bg-[radial-gradient(circle,rgba(255,255,255,0.05)_0%,transparent_70%)] rounded-full pointer-events-none -translate-x-1/2 -translate-y-1/2"
      ></div>
      <div class="flex items-center gap-3 sm:gap-4 z-10">
        <a
          href="/"
          class="text-[var(--text-muted)] hover:text-[var(--text-main)] transition flex items-center gap-2 bg-[var(--surface-bg)] px-4 py-2 rounded-xl border border-[var(--glass-border)] hover:bg-[var(--text-main)]/10"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"><path d="m15 18-6-6 6-6" /></svg
          >
          Back
        </a>
        <h1
          class="text-2xl sm:text-3xl font-bold tracking-tight text-[var(--text-main)] drop-shadow-lg"
        >
          Settings
        </h1>
      </div>

      <div class="flex items-center gap-3 w-full md:w-auto justify-end z-10">
        {#if saveStatus}
          <span
            class="text-green-400 text-sm font-medium bg-green-900/20 px-3 py-1 rounded-full border border-green-500/30"
            transition:fade>{saveStatus}</span
          >
        {/if}
        <button
          type="button"
          onclick={saveConfig}
          class="bg-blue-600 hover:bg-blue-500 shadow-[0_0_20px_rgba(37,99,235,0.4)] text-white px-6 py-2.5 rounded-xl font-medium transition flex items-center gap-2 border border-blue-400/30 w-full sm:w-auto justify-center"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="18"
            height="18"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            ><path
              d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z"
            /><polyline points="17 21 17 13 7 13 7 21" /><polyline
              points="7 3 7 8 15 8"
            /></svg
          >
          Save
        </button>
      </div>
    </header>

    {#if loading}
      <div
        class="flex items-center justify-center min-h-[260px] h-[40vh] glass-card rounded-3xl"
      >
        <div class="flex flex-col items-center gap-4">
          <div
            class="w-10 h-10 border-4 border-blue-500/30 border-t-blue-500 rounded-full animate-spin"
          ></div>
          <div
            class="text-xl font-medium text-[var(--text-muted)] animate-pulse"
          >
            Loading settings...
          </div>
        </div>
      </div>
    {:else if config}
      <div
        class="glass-card rounded-3xl overflow-hidden flex flex-col md:flex-row flex-1 shadow-2xl relative min-h-0"
      >
        <div
          class="absolute inset-0 bg-gradient-to-br from-white/5 to-transparent pointer-events-none"
        ></div>

        <!-- Sidebar Tabs -->
        <div
          class="settings-tabs md:w-60 lg:w-64 bg-black/10 p-2 sm:p-3 md:p-4 flex md:flex-col gap-1 md:gap-2 border-b md:border-b-0 md:border-r border-white/10 relative z-10 overflow-x-auto md:overflow-x-visible flex-shrink-0"
        >
          {#each tabs as tab}
            <button
              type="button"
              class="text-left px-4 py-2.5 md:py-3 rounded-xl transition-all duration-300 flex items-center gap-2.5 whitespace-nowrap {activeTab ===
              tab.id
                ? 'bg-[var(--text-main)]/10 text-[var(--text-main)] font-semibold shadow-[0_0_15px_var(--glass-shadow)] border border-[var(--glass-border)] md:translate-x-1'
                : 'text-[var(--text-muted)] hover:text-[var(--text-main)] hover:bg-[var(--text-main)]/5 border border-transparent md:hover:translate-x-1'}"
              onclick={() => (activeTab = tab.id)}
            >
              <div
                class="w-1.5 h-1.5 rounded-full transition-colors {activeTab ===
                tab.id
                  ? 'bg-blue-400 shadow-[0_0_8px_rgba(96,165,250,0.8)]'
                  : 'bg-transparent'}"
              ></div>
              {tab.label}
            </button>
          {/each}
        </div>

        <!-- Content Area -->
        <div
          class="flex-1 min-h-0 p-3 sm:p-4 md:p-6 lg:p-7 bg-transparent overflow-y-auto"
        >
          {#if activeTab === "location"}
            <div in:fade={{ duration: 200 }}>
              <h2 class="text-xl font-bold mb-6 flex items-center gap-2">
                <span class="text-2xl">📍</span> Locations
              </h2>

              <div class="mb-8 space-y-4">
                {#if config.locations.length === 0}
                  <div
                    class="text-[var(--text-muted)] italic p-8 border border-dashed border-white/10 rounded-xl text-center bg-white/5"
                  >
                    No locations added yet. Search below to add one.
                  </div>
                {:else}
                  {#each config.locations as loc, i}
                    <div
                      class="flex items-center justify-between p-4 rounded-xl transition border group {config.current_location_index ===
                      i
                        ? 'bg-blue-500/10 border-blue-500/50 shadow-[0_0_15px_rgba(59,130,246,0.1)]'
                        : 'bg-white/5 border-white/5 hover:border-white/20'}"
                    >
                      <button
                        type="button"
                        class="flex-1 text-left"
                        onclick={() => (config!.current_location_index = i)}
                      >
                        <div class="flex items-center gap-2">
                          <span class="font-medium text-lg">{loc.name}</span>
                          {#if config.current_location_index === i}
                            <span
                              class="text-xs bg-blue-500/20 text-blue-300 px-2 py-0.5 rounded-full border border-blue-500/30"
                              >Active</span
                            >
                          {/if}
                        </div>
                        <div
                          class="text-[var(--text-muted)] text-sm font-mono mt-1 flex gap-4"
                        >
                          <span>Lat: {loc.coordinates.latitude.toFixed(4)}</span
                          >
                          <span
                            >Lng: {loc.coordinates.longitude.toFixed(4)}</span
                          >
                        </div>
                      </button>
                      <button
                        type="button"
                        class="text-white/30 hover:text-red-400 p-2 transition opacity-70 md:opacity-0 md:group-hover:opacity-100"
                        onclick={() => removeLocation(i)}
                        aria-label="Remove location"
                        title="Remove Location"
                      >
                        <svg
                          xmlns="http://www.w3.org/2000/svg"
                          width="20"
                          height="20"
                          viewBox="0 0 24 24"
                          fill="none"
                          stroke="currentColor"
                          stroke-width="2"
                          stroke-linecap="round"
                          stroke-linejoin="round"
                          ><path d="M18 6 6 18" /><path d="m6 6 18 12" /></svg
                        >
                      </button>
                    </div>
                  {/each}
                {/if}
              </div>

              <h3 class="text-lg font-semibold mb-4">Add New Location</h3>
              <div class="flex flex-col sm:flex-row gap-2 mb-4 relative">
                <label for="city-search" class="sr-only">Search city</label>
                <input
                  id="city-search"
                  type="text"
                  bind:value={searchQuery}
                  placeholder="Search city (e.g. London, Mecca)..."
                  class="flex-1 bg-[var(--glass-bg)] border border-[var(--glass-border)] rounded-lg px-4 py-3 text-[var(--text-main)] placeholder-[var(--text-muted)] focus:outline-none focus:border-blue-500 transition focus:bg-[var(--text-main)]/5"
                  onkeydown={(e) => e.key === "Enter" && searchCity()}
                />
                <button
                  type="button"
                  onclick={searchCity}
                  disabled={searching || searchQuery.length < 3}
                  class="bg-blue-600/80 hover:bg-blue-600 disabled:opacity-50 px-6 py-2 rounded-lg transition font-medium min-w-[100px] w-full sm:w-auto"
                >
                  {searching ? "..." : "Search"}
                </button>
              </div>

              {#if searchResults.length > 0}
                <div
                  class="bg-black/40 rounded-xl overflow-hidden border border-white/10 max-h-60 overflow-y-auto shadow-xl"
                >
                  {#each searchResults as city}
                    <button
                      type="button"
                      class="w-full text-left px-4 py-3 hover:bg-white/10 transition border-b border-white/5 last:border-0 flex justify-between items-center group"
                      onclick={() => addLocation(city)}
                    >
                      <span>{city.name}, {city.country}</span>
                      <span
                        class="text-blue-400 opacity-0 group-hover:opacity-100 transition text-sm font-medium bg-blue-500/10 px-2 py-1 rounded"
                        >Add +</span
                      >
                    </button>
                  {/each}
                </div>
              {/if}
            </div>
          {:else if activeTab === "calculation"}
            <div in:fade={{ duration: 200 }} class="space-y-8">
              <div>
                <h2 class="text-xl font-bold mb-4 flex items-center gap-2">
                  <span class="text-2xl">🧮</span> Calculation Method
                </h2>
                <div class="relative">
                  <select
                    bind:value={config.calculation_method}
                    class="w-full bg-[var(--glass-bg)] border border-[var(--glass-border)] rounded-lg px-4 py-3 text-[var(--text-main)] focus:outline-none focus:border-blue-500 transition appearance-none cursor-pointer hover:bg-[var(--text-main)]/5"
                  >
                    {#each calculationMethods as method}
                      <option
                        value={method}
                        class={$theme === "light"
                          ? "bg-white text-slate-900"
                          : "bg-slate-800 text-white"}>{method}</option
                      >
                    {/each}
                  </select>
                  <div
                    class="absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none text-[var(--text-muted)]"
                  >
                    ▼
                  </div>
                </div>
              </div>

              <div>
                <h2 class="text-xl font-bold mb-4">Asr Madhab</h2>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                  {#each asrMadhabs as madhab}
                    <label
                      class="flex items-center gap-3 p-4 bg-[var(--glass-bg)] rounded-xl border border-[var(--glass-border)] cursor-pointer hover:bg-[var(--text-main)]/5 transition {config.asr_madhab ===
                      madhab
                        ? 'border-blue-500/50 bg-blue-500/5'
                        : ''}"
                    >
                      <div class="relative flex items-center">
                        <input
                          type="radio"
                          bind:group={config.asr_madhab}
                          value={madhab}
                          class="peer appearance-none w-5 h-5 border-2 border-white/30 rounded-full checked:border-blue-500 checked:bg-blue-500 transition-colors"
                        />
                        <div
                          class="absolute inset-0 m-auto w-2 h-2 rounded-full bg-white transform scale-0 peer-checked:scale-100 transition-transform"
                        ></div>
                      </div>
                      <span class="font-medium"
                        >{madhab === "Shafi"
                          ? "Standard (Shafi/Maliki/Hanbali)"
                          : "Hanafi"}</span
                      >
                    </label>
                  {/each}
                </div>
              </div>

              <div>
                <h2 class="text-xl font-bold mb-4">High Latitude Rule</h2>
                <div class="relative">
                  <select
                    bind:value={config.high_latitude_rule}
                    class="w-full bg-[var(--glass-bg)] border border-[var(--glass-border)] rounded-lg px-4 py-3 text-[var(--text-main)] focus:outline-none focus:border-blue-500 transition appearance-none cursor-pointer hover:bg-[var(--text-main)]/5"
                  >
                    {#each highLatitudeRules as rule}
                      <option
                        value={rule}
                        class={$theme === "light"
                          ? "bg-white text-slate-900"
                          : "bg-slate-800 text-white"}
                        >{rule.replace(/([A-Z])/g, " $1").trim()}</option
                      >
                    {/each}
                  </select>
                  <div
                    class="absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none text-[var(--text-muted)]"
                  >
                    ▼
                  </div>
                </div>
              </div>

              <div>
                <h2 class="text-xl font-bold mb-4">
                  Manual Adjustments (minutes)
                </h2>
                <div
                  class="grid grid-cols-1 sm:grid-cols-2 md:grid-cols-3 gap-4"
                >
                  {#each Object.entries(config.prayer_adjustments) as [prayer]}
                    {@const adjustmentId = `adjustment-${prayer}`}
                    <div
                      class="bg-[var(--glass-bg)] p-4 rounded-xl border border-[var(--glass-border)] hover:bg-[var(--text-main)]/5 transition"
                    >
                      <label
                        for={adjustmentId}
                        class="block text-[var(--text-muted)] text-sm mb-2 capitalize font-medium"
                        >{prayer}
                        <input
                          id={adjustmentId}
                          type="number"
                          bind:value={
                            config.prayer_adjustments[
                              prayer as keyof typeof config.prayer_adjustments
                            ]
                          }
                          class="w-full bg-[var(--text-main)]/5 border border-[var(--glass-border)] rounded-lg px-3 py-2 text-[var(--text-main)] text-center focus:border-blue-500 outline-none focus:bg-[var(--text-main)]/10 transition"
                        />
                      </label>
                    </div>
                  {/each}
                </div>
              </div>
            </div>
          {:else if activeTab === "reminders"}
            <div in:fade={{ duration: 200 }}>
              <h2 class="text-xl font-bold mb-2 flex items-center gap-2">
                <span class="text-2xl">🔔</span> Prayer Reminders
              </h2>
              <p class="text-[var(--text-muted)] mb-6">
                Configure notifications and sounds for each prayer.
              </p>
              {#if previewError}
                <p
                  class="mb-4 text-sm font-medium text-red-300 bg-red-500/10 border border-red-500/30 rounded-lg px-3 py-2"
                >
                  {previewError}
                </p>
              {/if}

              <div class="space-y-5">
                {#each prayerOrder.filter((p) => p in config!.reminders) as prayer}
                  {@const settings = config!.reminders[prayer]}
                  {@const reminderMinutesId = `reminder-minutes-${prayer}`}
                  <div
                    class="bg-[var(--glass-bg)] rounded-2xl border border-[var(--glass-border)] overflow-hidden transition hover:bg-[var(--text-main)]/5 shadow-[0_18px_45px_rgba(6,14,30,0.18)]"
                  >
                    <button
                      type="button"
                      class="w-full px-5 py-4 lg:px-6 lg:py-5 flex items-center justify-between bg-transparent"
                      onclick={() => (settings.enabled = !settings.enabled)}
                    >
                      <span
                        class="capitalize font-bold text-lg flex items-center gap-2 text-[var(--text-main)]"
                      >
                        {prayer}
                      </span>
                      <div
                        class="relative inline-flex items-center pointer-events-none"
                      >
                        <input
                          type="checkbox"
                          checked={settings.enabled}
                          tabindex="-1"
                          class="sr-only peer"
                        />
                        <div
                          class="w-11 h-6 bg-[var(--text-main)]/10 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full rtl:peer-checked:after:-translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:start-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-blue-600"
                        ></div>
                      </div>
                    </button>

                    {#if settings.enabled}
                      <div
                        class="p-4 lg:p-6 space-y-4 lg:space-y-5 border-t border-[var(--glass-border)] bg-[var(--text-main)]/5"
                        transition:fade
                      >
                        <div class="grid gap-4 lg:gap-5">
                          <!-- Before Reminder -->
                          <div
                            class="settings-reminder-panel"
                          >
                            <div class="min-w-0">
                              <label
                                class="block"
                                for={`reminder-before-${prayer}`}
                              >
                                <span
                                  class="block text-sm font-semibold text-[var(--text-main)] mb-1"
                                  >Before Prayer (Minutes)</span
                                >
                                <span class="block text-xs text-[var(--text-muted)] mb-3">
                                  Show a reminder before the prayer time.
                                </span>
                                <input
                                  id={`reminder-before-${prayer}`}
                                  type="number"
                                  bind:value={settings.minutes_before}
                                  min="0"
                                  max="60"
                                  disabled={!settings.before_enabled}
                                  class="w-full bg-[var(--glass-bg)] border border-[var(--glass-border)] rounded-lg px-3 py-2.5 text-[var(--text-main)] focus:border-blue-500 outline-none"
                                />
                              </label>
                            </div>
                            <div class="settings-reminder-options">
                              <label
                                class="settings-toggle-row"
                              >
                                <input
                                  type="checkbox"
                                  bind:checked={settings.before_enabled}
                                  class="accent-blue-500 w-4 h-4 rounded"
                                />
                                <span class="text-sm">Enable Before Reminder</span>
                              </label>
                              <label
                                class="settings-toggle-row"
                              >
                                <input
                                  type="checkbox"
                                  bind:checked={settings.play_sound_before}
                                  disabled={!settings.before_enabled}
                                  class="accent-blue-500 w-4 h-4 rounded"
                                />
                                <span class="text-sm">Play Alert Sound (Beep)</span>
                              </label>
                            </div>
                          </div>

                          <!-- On Time Reminder -->
                          <div
                            class="settings-reminder-panel"
                          >
                            <div class="min-w-0">
                              <span
                                class="block text-sm font-semibold text-[var(--text-main)] mb-1"
                                >On Time (Adhan)</span
                              >
                              <span class="block text-xs text-[var(--text-muted)]"
                                >Audio file options below when enabled</span
                              >
                            </div>
                            <div class="settings-reminder-options">
                              <label
                                class="settings-toggle-row"
                              >
                                <input
                                  type="checkbox"
                                  bind:checked={settings.show_notification}
                                  class="accent-blue-500 w-4 h-4 rounded"
                                />
                                <span class="text-sm">Show App Popup</span>
                              </label>
                              <label
                                class="settings-toggle-row"
                              >
                                <input
                                  type="checkbox"
                                  bind:checked={settings.play_adhan}
                                  class="accent-blue-500 w-4 h-4 rounded"
                                />
                                <span class="text-sm">Play Audio (Adhan)</span>
                              </label>
                            </div>
                          </div>

                          <!-- After Reminder -->
                          <div
                            class="settings-reminder-panel"
                          >
                            <div class="min-w-0">
                              <label
                                class="block"
                                for={`reminder-after-${prayer}`}
                              >
                                <span
                                  class="block text-sm font-semibold text-[var(--text-main)] mb-1"
                                  >After Prayer (Minutes)</span
                                >
                                <span class="block text-xs text-[var(--text-muted)] mb-3">
                                  Keep a post-prayer reminder active for a short window.
                                </span>
                                <input
                                  id={`reminder-after-${prayer}`}
                                  type="number"
                                  bind:value={settings.minutes_after}
                                  min="0"
                                  max="60"
                                  disabled={!settings.after_enabled}
                                  class="w-full bg-[var(--glass-bg)] border border-[var(--glass-border)] rounded-lg px-3 py-2.5 text-[var(--text-main)] focus:border-blue-500 outline-none"
                                />
                              </label>
                            </div>
                            <div class="settings-reminder-options">
                              <label
                                class="settings-toggle-row"
                              >
                                <input
                                  type="checkbox"
                                  bind:checked={settings.after_enabled}
                                  class="accent-blue-500 w-4 h-4 rounded"
                                />
                                <span class="text-sm">Enable After Reminder</span>
                              </label>
                              <label
                                class="settings-toggle-row"
                              >
                                <input
                                  type="checkbox"
                                  bind:checked={settings.play_sound_after}
                                  disabled={!settings.after_enabled}
                                  class="accent-blue-500 w-4 h-4 rounded"
                                />
                                <span class="text-sm">Play Alert Sound (Beep)</span>
                              </label>
                            </div>
                          </div>
                        </div>

                        <!-- Reminder Beep Sound File (for before/after) -->
                        {#if reminderPreviewError}
                          <p class="text-sm font-medium text-red-300 bg-red-500/10 border border-red-500/30 rounded-lg px-3 py-2">
                            {reminderPreviewError}
                          </p>
                        {/if}
                        <div
                          class="settings-sound-panel border-amber-500/20"
                          transition:fade
                        >
                          <div class="flex-1 min-w-0">
                            <div class="text-xs text-amber-400/80 uppercase tracking-wider mb-1 font-semibold">
                              Reminder Sound File
                            </div>
                            <div
                              class="truncate text-sm font-mono text-[var(--text-main)] bg-[var(--text-main)]/5 p-2 rounded border border-[var(--glass-border)]"
                              title={settings.custom_reminder_sound || "Default Classic Alarm"}
                            >
                              {settings.custom_reminder_sound
                                ? settings.custom_reminder_sound.split(/[\\/]/).pop()
                                : "Default Classic Alarm"}
                            </div>
                          </div>
                          <div class="settings-action-row">
                            <button
                              type="button"
                              class="px-4 py-2 bg-amber-500/10 text-amber-300 hover:bg-amber-500/20 rounded-lg text-sm transition font-medium border border-amber-500/20 w-full sm:w-auto text-center"
                              onclick={() => toggleReminderPreview(prayer)}
                              disabled={reminderPreviewBusy}
                            >{isPreviewingReminder(prayer) ? "Stop" : "Preview"}</button>
                            <button
                              type="button"
                              class="px-4 py-2 bg-[var(--text-main)]/5 hover:bg-[var(--text-main)]/10 text-[var(--text-main)] rounded-lg text-sm transition font-medium border border-[var(--glass-border)] w-full sm:w-auto text-center"
                              onclick={() => selectReminderFile(prayer)}
                            >Browse...</button>
                            {#if settings.custom_reminder_sound}
                              <button
                                type="button"
                                class="px-4 py-2 bg-red-500/10 text-red-400 hover:bg-red-500/20 rounded-lg text-sm transition font-medium border border-red-500/20 w-full sm:w-auto text-center"
                                onclick={() => clearReminderFile(prayer)}
                              >Reset</button>
                            {/if}
                            {#if isPreviewingReminder(prayer)}
                              <span class="px-3 py-2 text-xs font-semibold rounded-lg border border-amber-500/30 bg-amber-500/10 text-amber-300 w-full sm:w-auto text-center">
                                Preview playing
                              </span>
                            {/if}
                          </div>
                        </div>

                        {#if settings.play_adhan}
                          <div
                            class="settings-sound-panel"
                            transition:fade
                          >
                            <div class="flex-1 min-w-0">
                              <div
                                class="text-xs text-[var(--text-muted)] uppercase tracking-wider mb-1 font-semibold"
                              >
                                Custom Sound File
                              </div>
                              <div
                                class="truncate text-sm font-mono text-[var(--text-main)] bg-[var(--text-main)]/5 p-2 rounded border border-[var(--glass-border)]"
                                title={settings.custom_sound || "Default"}
                              >
                                {settings.custom_sound
                                  ? settings.custom_sound.split(/[\\/]/).pop()
                                  : "Default App Sound"}
                              </div>
                            </div>
                            <div class="settings-action-row">
                              <button
                                type="button"
                                class="px-4 py-2 bg-blue-500/10 text-blue-400 hover:bg-blue-500/20 rounded-lg text-sm transition font-medium border border-blue-500/20 w-full sm:w-auto text-center"
                                onclick={() => togglePreview(prayer)}
                                disabled={previewBusy}
                                >{getPreviewActionLabel(prayer)}</button
                              >
                              <button
                                type="button"
                                class="px-4 py-2 bg-amber-500/10 text-amber-300 hover:bg-amber-500/20 rounded-lg text-sm transition font-medium border border-amber-500/20 disabled:opacity-40 disabled:cursor-not-allowed w-full sm:w-auto text-center"
                                onclick={stopPreview}
                                disabled={!isPreviewing(prayer) || previewBusy}
                                >Stop</button
                              >
                              <button
                                type="button"
                                class="px-4 py-2 bg-[var(--text-main)]/5 hover:bg-[var(--text-main)]/10 text-[var(--text-main)] rounded-lg text-sm transition font-medium border border-[var(--glass-border)] w-full sm:w-auto text-center"
                                onclick={() => selectAdhanFile(prayer)}
                                >Browse...</button
                              >
                              {#if settings.custom_sound}
                                <button
                                  type="button"
                                  class="px-4 py-2 bg-red-500/10 text-red-400 hover:bg-red-500/20 rounded-lg text-sm transition font-medium border border-red-500/20 w-full sm:w-auto text-center"
                                  onclick={() => clearAdhanFile(prayer)}
                                  >Reset</button
                                >
                              {/if}
                              {#if isPreviewing(prayer)}
                                <span
                                  class="px-3 py-2 text-xs font-semibold rounded-lg border border-emerald-500/30 bg-emerald-500/10 text-emerald-300 w-full sm:w-auto text-center"
                                >
                                  {previewPaused
                                    ? "Preview paused"
                                    : "Preview playing"}
                                </span>
                              {/if}
                            </div>
                          </div>
                        {/if}
                      </div>
                    {/if}
                  </div>
                {/each}
              </div>
            </div>
          {:else if activeTab === "appearance"}
            <div in:fade={{ duration: 200 }} class="space-y-8">
              <div>
                <h2 class="text-xl font-bold mb-4 flex items-center gap-2">
                  <span class="text-2xl">🎨</span> Theme
                </h2>
                <div class="flex flex-col sm:flex-row gap-3 sm:gap-4">
                  {#each ["system", "light", "dark"] as theme}
                    <label
                      class="flex items-center gap-3 p-4 bg-[var(--glass-bg)] rounded-xl border border-[var(--glass-border)] cursor-pointer hover:bg-[var(--text-main)]/5 transition flex-1 {config
                        .appearance.theme === theme
                        ? 'border-blue-500/50 bg-blue-500/5'
                        : ''}"
                    >
                      <div class="relative flex items-center">
                        <input
                          type="radio"
                          bind:group={config.appearance.theme}
                          value={theme}
                          class="peer appearance-none w-5 h-5 border-2 border-[var(--text-muted)]/30 rounded-full checked:border-blue-500 checked:bg-blue-500 transition-colors"
                        />
                        <div
                          class="absolute inset-0 m-auto w-2 h-2 rounded-full bg-white transform scale-0 peer-checked:scale-100 transition-transform"
                        ></div>
                      </div>
                      <span
                        class="capitalize font-medium text-[var(--text-main)]"
                        >{theme}</span
                      >
                    </label>
                  {/each}
                </div>
              </div>

              <div>
                <h2 class="text-xl font-bold mb-4">Visual mode</h2>
                <p class="text-sm text-[var(--text-muted)] mb-3 leading-relaxed">
                  Performance is the default. It skips backdrop blur and looping
                  animations so low-end GPUs keep desktop FPS. Glass is the old
                  look, opt-in only.
                </p>
                <div class="flex flex-col sm:flex-row gap-3 sm:gap-4">
                  {#each [
                    {
                      id: "performance" as VisualMode,
                      label: "Performance",
                      hint: "Solid surfaces, no compositor tax",
                    },
                    {
                      id: "glass" as VisualMode,
                      label: "Glass",
                      hint: "Blurred cards, heavier GPU",
                    },
                  ] as option}
                    <label
                      class="flex items-center gap-3 p-4 bg-[var(--glass-bg)] rounded-xl border border-[var(--glass-border)] cursor-pointer hover:bg-[var(--text-main)]/5 transition flex-1 {(config
                        .appearance.visual_mode || "performance") === option.id
                        ? 'border-blue-500/50 bg-blue-500/5'
                        : ''}"
                    >
                      <div class="relative flex items-center">
                        <input
                          type="radio"
                          bind:group={config.appearance.visual_mode}
                          value={option.id}
                          class="peer appearance-none w-5 h-5 border-2 border-[var(--text-muted)]/30 rounded-full checked:border-blue-500 checked:bg-blue-500 transition-colors"
                        />
                        <div
                          class="absolute inset-0 m-auto w-2 h-2 rounded-full bg-white transform scale-0 peer-checked:scale-100 transition-transform"
                        ></div>
                      </div>
                      <span class="flex flex-col">
                        <span class="font-medium text-[var(--text-main)]"
                          >{option.label}</span
                        >
                        <span class="text-xs text-[var(--text-muted)]"
                          >{option.hint}</span
                        >
                      </span>
                    </label>
                  {/each}
                </div>
              </div>

              <div>
                <h2 class="text-xl font-bold mb-4">Clock Format</h2>
                <div class="flex flex-col sm:flex-row gap-3 sm:gap-4">
                  {#each ["hour12", "hour24"] as format}
                    <label
                      class="flex items-center gap-3 p-4 bg-white/5 rounded-xl border border-white/5 cursor-pointer hover:bg-white/10 transition flex-1 {config
                        .appearance.clock_format === format
                        ? 'border-blue-500/50 bg-blue-500/5'
                        : ''}"
                    >
                      <div class="relative flex items-center">
                        <input
                          type="radio"
                          bind:group={config.appearance.clock_format}
                          value={format}
                          class="peer appearance-none w-5 h-5 border-2 border-white/30 rounded-full checked:border-blue-500 checked:bg-blue-500 transition-colors"
                        />
                        <div
                          class="absolute inset-0 m-auto w-2 h-2 rounded-full bg-white transform scale-0 peer-checked:scale-100 transition-transform"
                        ></div>
                      </div>
                      <span class="font-medium"
                        >{format === "hour12"
                          ? "12 Hour (AM/PM)"
                          : "24 Hour"}</span
                      >
                    </label>
                  {/each}
                </div>
              </div>

              <div
                class="p-4 bg-[var(--glass-bg)] rounded-xl border border-[var(--glass-border)] hover:bg-[var(--text-main)]/5 transition"
              >
                <label
                  class="flex items-center gap-3 cursor-pointer text-[var(--text-main)]"
                >
                  <input
                    type="checkbox"
                    bind:checked={config.appearance.show_arabic}
                    class="accent-blue-500 w-5 h-5 rounded"
                  />
                  <span class="font-medium">Show Arabic Names</span>
                </label>
              </div>

              <div class="space-y-3">
                <h2 class="text-xl font-bold flex items-center gap-2">
                  <span class="text-2xl">🌙</span> Hijri / Moon Sighting Alignment
                </h2>
                <p class="text-sm text-[var(--text-muted)] leading-relaxed">
                  Ramadan and Eid often shift by a day based on local moon
                  observation. Set a global fallback, or align the
                  <strong>current city</strong> automatically from an online
                  authority calendar (Aladhan). Per-city offset overrides the
                  global value.
                </p>
                <div
                  class="flex items-center gap-3 bg-[var(--glass-bg)] p-4 rounded-xl border border-[var(--glass-border)]"
                >
                  <button
                    type="button"
                    class="w-10 h-10 rounded-lg bg-[var(--text-main)]/10 hover:bg-[var(--text-main)]/20 text-[var(--text-main)] font-bold text-xl transition border border-[var(--glass-border)] disabled:opacity-30"
                    onclick={() => {
                      if (!config) return;
                      if (config.hijri_offset > -3) config.hijri_offset -= 1;
                      const loc = config.locations[config.current_location_index];
                      if (loc) loc.hijri_offset = config.hijri_offset;
                    }}
                    disabled={config.hijri_offset <= -3}
                    aria-label="Decrease Hijri offset">−</button
                  >
                  <div class="flex-1 text-center">
                    <span class="text-2xl font-bold text-[var(--text-main)]">
                      {config.hijri_offset > 0
                        ? `+${config.hijri_offset}`
                        : config.hijri_offset}
                    </span>
                    <p class="text-xs text-[var(--text-muted)] mt-0.5">
                      {config.hijri_offset === 0
                        ? "Tabular / astronomical baseline"
                        : config.hijri_offset < 0
                          ? `${Math.abs(config.hijri_offset)} day${Math.abs(config.hijri_offset) > 1 ? "s" : ""} earlier (local sighting)`
                          : `${config.hijri_offset} day${config.hijri_offset > 1 ? "s" : ""} later (local sighting)`}
                    </p>
                  </div>
                  <button
                    type="button"
                    class="w-10 h-10 rounded-lg bg-[var(--text-main)]/10 hover:bg-[var(--text-main)]/20 text-[var(--text-main)] font-bold text-xl transition border border-[var(--glass-border)] disabled:opacity-30"
                    onclick={() => {
                      if (!config) return;
                      if (config.hijri_offset < 3) config.hijri_offset += 1;
                      const loc = config.locations[config.current_location_index];
                      if (loc) loc.hijri_offset = config.hijri_offset;
                    }}
                    disabled={config.hijri_offset >= 3}
                    aria-label="Increase Hijri offset">+</button
                  >
                </div>
                <div class="flex flex-wrap items-center gap-3">
                  <button
                    type="button"
                    class="px-4 py-2 rounded-xl bg-emerald-600/80 hover:bg-emerald-500 text-white font-medium disabled:opacity-40"
                    disabled={alignBusy || !config.locations.length}
                    onclick={autoAlignHijriForCurrentCity}
                  >
                    {alignBusy ? "Aligning…" : "Auto-align current city"}
                  </button>
                  {#if config.locations[config.current_location_index]}
                    <label class="flex items-center gap-2 text-sm text-[var(--text-muted)]">
                      <input
                        type="checkbox"
                        checked={!!config.locations[config.current_location_index]
                          .hijri_auto_align}
                        onchange={(e) => {
                          const loc =
                            config.locations[config.current_location_index];
                          if (loc)
                            loc.hijri_auto_align = (
                              e.currentTarget as HTMLInputElement
                            ).checked;
                        }}
                      />
                      Keep auto-aligning this city daily
                    </label>
                  {/if}
                </div>
                {#if alignStatus}
                  <p class="text-sm text-[var(--text-muted)]">{alignStatus}</p>
                {/if}
              </div>
            </div>
          {:else if activeTab === "audio"}
            <div in:fade={{ duration: 200 }} class="space-y-8">
              <div>
                <h2 class="text-xl font-bold mb-4 flex items-center gap-2">
                  <span class="text-2xl">🔊</span> Audio Settings
                </h2>
                <div
                  class="bg-[var(--glass-bg)] p-6 rounded-xl border border-[var(--glass-border)]"
                >
                  <div class="flex justify-between mb-2">
                    <label
                      for="global-volume"
                      class="font-medium text-[var(--text-main)]"
                      >Global Volume</label
                    >
                    <span class="font-mono text-[var(--text-muted)]"
                      >{Math.round(config.audio.global_volume * 100)}%</span
                    >
                  </div>
                  <input
                    id="global-volume"
                    type="range"
                    min="0"
                    max="1"
                    step="0.05"
                    bind:value={config.audio.global_volume}
                    class="w-full accent-blue-500 h-2 bg-[var(--text-main)]/10 rounded-lg appearance-none cursor-pointer hover:bg-[var(--text-main)]/20 transition-colors"
                  />
                </div>
              </div>

              <div
                class="p-4 bg-white/5 rounded-xl border border-white/5 hover:bg-white/10 transition"
              >
                <label class="flex items-center gap-3 cursor-pointer">
                  <input
                    type="checkbox"
                    bind:checked={config.audio.adhan_for_all}
                    class="accent-blue-500 w-5 h-5 rounded"
                  />
                  <span class="font-medium"
                    >Use same Adhan sound for all prayers</span
                  >
                </label>
              </div>
            </div>
          {:else if activeTab === "advanced"}
            <div in:fade={{ duration: 200 }} class="space-y-8">
              <div>
                <h2 class="text-xl font-bold mb-4 flex items-center gap-2">
                  <span class="text-2xl">⚡</span> Startup & System
                </h2>
                <div
                  class="bg-[var(--glass-bg)] rounded-xl border border-[var(--glass-border)] divide-y divide-[var(--glass-border)] overflow-hidden"
                >
                  <label
                    class="flex items-center gap-3 p-4 cursor-pointer hover:bg-[var(--text-main)]/5 transition text-[var(--text-main)]"
                  >
                    <input
                      type="checkbox"
                      bind:checked={config.advanced.auto_start}
                      class="accent-blue-500 w-5 h-5 rounded"
                    />
                    <span class="font-medium">Start on System Startup</span>
                  </label>
                  <label
                    class="flex items-center gap-3 p-4 cursor-pointer hover:bg-[var(--text-main)]/5 transition text-[var(--text-main)]"
                  >
                    <input
                      type="checkbox"
                      bind:checked={config.advanced.start_minimized}
                      class="accent-blue-500 w-5 h-5 rounded"
                    />
                    <span class="font-medium">Start Minimized</span>
                  </label>
                  <label
                    class="flex items-center gap-3 p-4 cursor-pointer hover:bg-[var(--text-main)]/5 transition text-[var(--text-main)]"
                  >
                    <input
                      type="checkbox"
                      bind:checked={config.advanced.minimize_to_tray}
                      class="accent-blue-500 w-5 h-5 rounded"
                    />
                    <span class="font-medium"
                      >Minimize to Tray instead of Exit</span
                    >
                  </label>
                  <label
                    class="flex items-center gap-3 p-4 cursor-pointer hover:bg-[var(--text-main)]/5 transition text-[var(--text-main)]"
                  >
                    <input
                      type="checkbox"
                      bind:checked={config.advanced.auto_update_check}
                      class="accent-blue-500 w-5 h-5 rounded"
                    />
                    <span class="font-medium"
                      >Check for Updates Automatically</span
                    >
                  </label>
                </div>
              </div>
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .settings-frame {
    max-width: min(1540px, 100%);
  }

  .settings-tabs {
    scrollbar-width: none;
  }

  .settings-tabs::-webkit-scrollbar {
    display: none;
  }

  .settings-reminder-panel {
    display: grid;
    gap: 1rem;
    padding: 1rem;
    border-radius: 1rem;
    border: 1px solid var(--glass-border);
    background: color-mix(in srgb, var(--text-main) 6%, transparent);
  }

  .settings-reminder-options {
    display: grid;
    gap: 0.75rem;
    align-content: start;
  }

  .settings-toggle-row {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    cursor: pointer;
    padding: 0.75rem;
    border-radius: 0.875rem;
    color: var(--text-main);
    transition: background-color 150ms ease;
  }

  .settings-toggle-row:hover {
    background: color-mix(in srgb, var(--text-main) 6%, transparent);
  }

  .settings-sound-panel {
    display: grid;
    gap: 1rem;
    padding: 1rem;
    border-radius: 1rem;
    border: 1px solid var(--glass-border);
    background: var(--glass-bg);
  }

  .settings-action-row {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: flex-start;
  }

  @media (min-width: 1024px) {
    .settings-reminder-panel {
      grid-template-columns: minmax(0, 1.35fr) minmax(260px, 0.9fr);
      align-items: end;
      padding: 1.125rem 1.25rem;
    }

    .settings-sound-panel {
      grid-template-columns: minmax(0, 1fr) auto;
      align-items: center;
      padding: 1.125rem 1.25rem;
    }

    .settings-action-row {
      justify-content: flex-end;
    }
  }

  @media (max-height: 760px) {
    .settings-shell {
      padding-bottom: 0.75rem;
    }
  }
</style>
