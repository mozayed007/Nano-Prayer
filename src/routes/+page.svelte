<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { fade, fly } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { currentPrayer, clockFormat } from "$lib/stores";

  interface PrayerTimes {
    date: string;
    location_name: string | null;
    fajr: string;
    sunrise: string;
    dhuhr: string;
    asr: string;
    maghrib: string;
    isha: string;
    current_prayer: string | null;
    next_prayer: string | null;
    minutes_to_next: number | null;
    qibla_direction: number;
  }

  interface HijriDate {
    year: number;
    month: number;
    day: number;
    month_name: string;
    formatted: string;
    formatted_arabic: string;
  }

  let prayerTimes = $state<PrayerTimes | null>(null);
  let hijriDate = $state<HijriDate | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let currentTime = $state(new Date());
  let isFetching = false;

  // Hourglass flip state
  let lastPrayer = $state<string | null>(null);
  let rotationDegrees = $state(0);

  // Helper to convert HH:MM to minutes from midnight
  function timeStringToMinutes(timeStr: string): number {
    if (!timeStr || timeStr === "--:--") return 0;
    const [h, m] = timeStr.split(":").map(Number);
    return h * 60 + m;
  }

  // Hourglass sand computations
  let hourglassProgress = $derived.by(() => {
    if (
      !prayerTimes ||
      prayerTimes.minutes_to_next === null ||
      prayerTimes.minutes_to_next === undefined
    ) {
      return 0;
    }

    if (!prayerTimes.current_prayer || !prayerTimes.next_prayer) {
      return Math.max(
        0,
        Math.min(1, getProgress(prayerTimes.minutes_to_next) / 100),
      );
    }

    // Attempt to calculate total time between current (previous) and next prayer
    const currentKey =
      prayerTimes.current_prayer.toLowerCase() as keyof PrayerTimes;
    const nextKey = prayerTimes.next_prayer.toLowerCase() as keyof PrayerTimes;

    const currentTimeStr = prayerTimes[currentKey] as string;
    const nextTimeStr = prayerTimes[nextKey] as string;

    if (
      !currentTimeStr ||
      !nextTimeStr ||
      currentTimeStr === "--:--" ||
      nextTimeStr === "--:--"
    ) {
      return Math.max(
        0,
        Math.min(1, getProgress(prayerTimes.minutes_to_next) / 100),
      );
    }

    const currentMins = timeStringToMinutes(currentTimeStr);
    let nextMins = timeStringToMinutes(nextTimeStr);

    // If next prayer is earlier in the day than current (e.g., Isha to Fajr next day)
    if (nextMins <= currentMins) {
      nextMins += 24 * 60;
    }

    const totalMinutes = nextMins - currentMins;
    if (totalMinutes <= 0) return 1;

    // Elapsed time is total time minus the remaining time
    const elapsedMinutes = totalMinutes - prayerTimes.minutes_to_next;
    const progress = elapsedMinutes / totalMinutes;

    return Math.max(0, Math.min(1, progress));
  });

  $effect(() => {
    if (prayerTimes?.next_prayer && prayerTimes.next_prayer !== lastPrayer) {
      if (lastPrayer !== null) {
        rotationDegrees += 180;
      }
      lastPrayer = prayerTimes.next_prayer;
    }
  });

  const prayers = [
    "fajr",
    "sunrise",
    "dhuhr",
    "asr",
    "maghrib",
    "isha",
  ] as const;

  // Prayer icons (SVG paths)
  const prayerIcons: Record<string, string> = {
    fajr: "M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z", // Moon/Sun mix
    sunrise:
      "M12 2v2m0 16v2M4.93 4.93l1.41 1.41m11.32 11.32l1.41 1.41M2 12h2m16 0h2M6.34 17.66l-1.41 1.41M19.07 4.93l-1.41 1.41", // Sun
    dhuhr:
      "M12 7a5 5 0 1 0 0 10 5 5 0 0 0 0-10zm0-5v2m0 18v2M4.22 4.22l1.42 1.42m12.72 12.72l1.42 1.42M1 12h2m18 0h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42", // Full Sun
    asr: "M12 7a5 5 0 1 0 0 10 5 5 0 0 0 0-10zm0-5v2m0 18v2M4.22 4.22l1.42 1.42m12.72 12.72l1.42 1.42M1 12h2m18 0h2M4.22 19.78l1.42-1.42M18.36 5.64l1.42-1.42", // Sun (same as Dhuhr for now, maybe lower)
    maghrib: "M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z", // Sunset/Moon
    isha: "M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z", // Moon
  };

  async function loadData() {
    // Check if running in Tauri environment
    if (
      typeof window !== "undefined" &&
      !("__TAURI_INTERNALS__" in window) &&
      !("__TAURI__" in window)
    ) {
      error =
        "Tauri API not available. Please run with `npm run tauri:dev` and ensure the Tauri window is open.";
      loading = false;
      return;
    }

    if (isFetching) return;
    isFetching = true;

    try {
      const [times, hijri] = await Promise.all([
        invoke<PrayerTimes>("get_prayer_times", {}),
        invoke<HijriDate>("get_hijri_date", {}),
      ]);
      prayerTimes = times;
      hijriDate = hijri;
      error = null;
      $currentPrayer = times.next_prayer;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
      isFetching = false;
    }
  }

  function formatTime(date: Date): string {
    return date.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: $clockFormat !== "hour24",
    });
  }

  function formatPrayerTime(timeStr: string): string {
    if (!timeStr || timeStr === "--:--") return timeStr;
    const [h, m] = timeStr.split(":").map(Number);
    if ($clockFormat === "hour24") return timeStr;

    const hour = h % 12 || 12;
    const ampm = h >= 12 ? "PM" : "AM";
    return `${hour}:${m.toString().padStart(2, "0")} ${ampm}`;
  }

  function formatMinutes(minutes: number): string {
    const h = Math.floor(minutes / 60);
    const m = minutes % 60;
    if (h > 0) {
      return `${h}h ${m}m`;
    }
    return `${m}m`;
  }

  function getProgress(minutes: number | null): number {
    if (minutes === null) return 0;
    // Max is about 3 hours (180 min) for the visual ring
    return Math.max(0, Math.min(100, ((180 - minutes) / 180) * 100));
  }

  onMount(() => {
    loadData();

    // Load clock format from config
    invoke("get_config")
      .then((cfg: any) => {
        if (cfg?.appearance?.clock_format) {
          $clockFormat = cfg.appearance.clock_format;
        }
      })
      .catch(() => {});

    // Update current time every second
    const interval = setInterval(() => {
      currentTime = new Date();
    }, 1000);

    // Refresh prayer times every minute
    const refreshInterval = setInterval(loadData, 60000);

    return () => {
      clearInterval(interval);
      clearInterval(refreshInterval);
    };
  });
</script>

<svelte:head>
  <title>NanoPrayer</title>
</svelte:head>

{#if loading}
  <div
    class="flex items-center justify-center h-full min-h-[220px]"
    transition:fade
  >
    <div class="glass-card p-8 rounded-2xl">
      <p class="text-center text-lg text-[var(--text-main)]">
        Loading prayer times...
      </p>
    </div>
  </div>
{:else if error}
  <div
    class="flex items-center justify-center h-full min-h-[220px]"
    transition:fade
  >
    <div class="glass-card p-8 rounded-2xl border-red-500 border">
      <p class="text-center text-red-300 mb-4">Error: {error}</p>
      <button
        type="button"
        onclick={loadData}
        class="w-full bg-white/20 hover:bg-white/30 py-2 rounded-lg transition text-[var(--text-main)]"
        >Retry</button
      >
    </div>
  </div>
{:else}
  <div
    class="praytime-page flex flex-col gap-2 md:gap-3 h-full min-h-0 max-w-6xl mx-auto w-full pt-1 pb-2 md:pb-4 overflow-hidden"
    in:fade={{ duration: 500 }}
  >
    <!-- Header -->
    <header
      class="flex flex-col gap-3 md:gap-4 md:flex-row md:justify-between md:items-end px-3 sm:px-4 md:px-8 relative mb-1 md:mb-2 flex-shrink-0"
    >
      <div class="z-10">
        <h1
          class="city-title text-4xl sm:text-5xl lg:text-6xl font-black tracking-tighter text-[var(--text-main)] drop-shadow-sm"
        >
          {prayerTimes?.location_name || "Location"}
        </h1>
        <p
          class="text-[var(--text-muted)] mt-2 font-medium tracking-wider uppercase text-xs md:text-sm"
        >
          {currentTime.toLocaleDateString("en-US", {
            weekday: "long",
            year: "numeric",
            month: "long",
            day: "numeric",
          })}
        </p>
      </div>
      <div class="text-right z-10 hidden md:block md:self-start lg:self-auto">
        <p
          class="text-xl lg:text-3xl font-bold tracking-tight text-[var(--text-main)]/90"
        >
          {hijriDate?.formatted}
        </p>
        <p
          class="text-[var(--text-muted)] font-arabic text-base lg:text-xl mt-1"
        >
          {hijriDate?.formatted_arabic}
        </p>
      </div>
    </header>

    <div
      class="grid grid-cols-1 lg:grid-cols-12 gap-4 md:gap-6 lg:flex-1 min-h-0"
    >
      <!-- Left Column: Timer & Info -->
      <div
        class="flex flex-col gap-3 md:gap-4 lg:col-span-6 xl:col-span-7 h-full min-h-0"
      >
        <!-- Countdown Card -->
        <div
          class="countdown-card backdrop-blur-3xl bg-[var(--glass-bg)] border border-[var(--glass-border)] p-3 md:p-5 lg:p-6 rounded-[1.5rem] md:rounded-[2rem] flex flex-col items-center justify-center relative overflow-hidden flex-1 min-h-[160px] shadow-[0_8px_32px_var(--glass-shadow)] group"
        >
          <div
            class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[150%] md:w-[120%] aspect-square bg-[radial-gradient(circle,rgba(255,255,255,0.1)_0%,transparent_70%)] opacity-50 group-hover:opacity-100 transition-opacity duration-1000 pointer-events-none clamp-size"
          ></div>

          <div
            class="relative z-10 text-center flex flex-col items-center w-full"
          >
            <div
              class="bg-[var(--glass-bg)] border border-[var(--glass-border)] px-4 sm:px-6 py-1.5 sm:py-2 rounded-full backdrop-blur-md mb-2 md:mb-3 lg:mb-5 shadow-inner"
            >
              <span
                class="text-base sm:text-lg md:text-xl font-mono font-medium tracking-widest text-[var(--text-main)]/90 drop-shadow-[0_0_8px_rgba(255,255,255,0.1)]"
              >
                {formatTime(currentTime)}
              </span>
            </div>

            {#if prayerTimes?.next_prayer && prayerTimes?.minutes_to_next !== null}
              <div
                class="relative flex items-center justify-center transition-transform duration-1000 ease-in-out"
                style="transform: rotate({rotationDegrees}deg) scale(1);"
              >
                <!-- Enhanced Progress Ring / Circular Hourglass -->
                <svg
                  class="ring-visual w-[60vw] sm:w-[50vw] md:w-[45vw] lg:w-[40vw] max-w-[340px] aspect-square drop-shadow-[0_0_30px_rgba(96,165,250,0.15)] max-h-[60vh] p-2 sm:p-4"
                  viewBox="0 0 288 288"
                >
                  <defs>
                    <linearGradient
                      id="sand-gradient"
                      x1="0%"
                      y1="0%"
                      x2="0%"
                      y2="100%"
                    >
                      <stop offset="0%" stop-color="#3b82f6" />
                      <stop offset="100%" stop-color="#8b5cf6" />
                    </linearGradient>
                    <clipPath id="circle-clip">
                      <circle cx="144" cy="144" r="130" />
                    </clipPath>
                  </defs>

                  <!-- Ring Outline -->
                  <circle
                    cx="144"
                    cy="144"
                    r="130"
                    stroke="currentColor"
                    stroke-width="2"
                    fill="transparent"
                    class="text-[var(--text-main)]/10"
                  />

                  <!-- Sand Accumulation (Bottom of circle) -->
                  <rect
                    x="14"
                    y={274 - 130 * 2 * hourglassProgress}
                    width="260"
                    height={130 * 2 * hourglassProgress}
                    fill="url(#sand-gradient)"
                    opacity="0.2"
                    clip-path="url(#circle-clip)"
                    class="transition-all duration-1000"
                  />

                  <!-- Progress Path (Outer Border) -->
                  <circle
                    cx="144"
                    cy="144"
                    r="130"
                    stroke="url(#sand-gradient)"
                    stroke-width="8"
                    fill="transparent"
                    stroke-linecap="round"
                    class="transition-all duration-1000 ease-out -rotate-90 origin-center"
                    stroke-dasharray={2 * Math.PI * 130}
                    stroke-dashoffset={2 *
                      Math.PI *
                      130 *
                      (1 - hourglassProgress)}
                  />
                </svg>

                <div
                  class="absolute inset-0 flex flex-col items-center justify-center pointer-events-none"
                >
                  <span
                    class="minutes-text text-3xl sm:text-4xl md:text-5xl lg:text-6xl font-black tracking-tighter text-[var(--text-main)] drop-shadow-lg"
                    >{formatMinutes(prayerTimes.minutes_to_next)}</span
                  >
                  <span
                    class="text-[var(--text-muted)] text-[9px] sm:text-[10px] md:text-xs uppercase tracking-[0.2em] mt-1 font-bold"
                    >Until {prayerTimes.next_prayer}</span
                  >
                </div>
              </div>

              <div
                class="mt-2 md:mt-3 lg:mt-5 flex items-center justify-center gap-2 bg-[var(--glass-bg)] px-4 py-1.5 md:py-2 rounded-xl border border-[var(--glass-border)] backdrop-blur-md w-full max-w-[14rem] shadow-inner flex-shrink-0"
              >
                <div
                  class="w-2.5 h-2.5 rounded-full {prayerTimes.current_prayer
                    ? 'bg-emerald-400 animate-pulse shadow-[0_0_10px_rgba(52,211,153,0.8)]'
                    : 'bg-[var(--text-main)]/30'}"
                ></div>
                <p class="text-[var(--text-muted)] font-medium tracking-wide">
                  Current: <span
                    class="text-[var(--text-main)] font-bold capitalize"
                    >{prayerTimes.current_prayer || "None"}</span
                  >
                </p>
              </div>
            {/if}
          </div>
        </div>
      </div>

      <!-- Right Column: Prayer List -->
      <div
        class="daily-card backdrop-blur-2xl bg-[var(--glass-bg)] border border-[var(--glass-border)] p-2 sm:p-4 md:p-5 lg:p-6 rounded-[1.5rem] md:rounded-[2rem] flex flex-col lg:col-span-6 xl:col-span-5 relative overflow-hidden shadow-[0_8px_32px_var(--glass-shadow)] flex-1 min-h-[0px]"
      >
        <div
          class="absolute right-0 top-0 w-[120%] lg:w-[100%] aspect-square bg-[radial-gradient(circle,rgba(255,255,255,0.05)_0%,transparent_70%)] pointer-events-none -translate-y-1/2"
        ></div>

        <h2
          class="text-lg sm:text-xl font-bold tracking-tight mb-2 md:mb-4 px-1 md:px-2 text-[var(--text-main)]/90 flex-shrink-0"
        >
          Daily Times
        </h2>

        <div
          class="space-y-1.5 md:space-y-2 relative z-10 flex-1 overflow-y-auto pr-1"
        >
          {#each prayers as prayer, i}
            {@const time = prayerTimes?.[prayer] || "--:--"}
            {@const isCurrent =
              prayerTimes?.current_prayer?.toLowerCase() === prayer}
            {@const isNext = prayerTimes?.next_prayer?.toLowerCase() === prayer}

            <div
              class="flex items-center justify-between p-2.5 sm:p-3 md:p-4 rounded-2xl transition-all duration-500 ease-out group relative overflow-hidden min-h-0
                 {isCurrent
                ? 'bg-[var(--text-main)]/10 shadow-[0_4px_24px_var(--glass-shadow)] border border-[var(--glass-border)] scale-[1.02]'
                : 'bg-[var(--text-main)]/5 border border-transparent hover:bg-[var(--text-main)]/10 hover:border-[var(--glass-border)]'} 
                 {isNext ? 'bg-blue-500/10 border-blue-500/30' : ''}"
              style="transition-delay: {i * 40}ms"
              in:fly={{ x: 20, duration: 600, delay: i * 80 }}
            >
              <!-- Inner glow for current prayer -->
              {#if isCurrent}
                <div
                  class="absolute inset-0 bg-gradient-to-r from-white/10 to-transparent opacity-50 pointer-events-none"
                ></div>
              {/if}

              <div
                class="flex items-center gap-2 sm:gap-4 md:gap-5 relative z-10 min-w-0"
              >
                <div
                  class="w-8 h-8 sm:w-12 sm:h-12 md:w-14 md:h-14 flex items-center justify-center rounded-xl transition-colors duration-300
                    {isCurrent
                    ? 'bg-[var(--text-main)] text-[var(--on-text-main)] shadow-lg shadow-[var(--text-main)]/20'
                    : 'bg-[var(--text-main)]/5 text-[var(--text-main)]/60 group-hover:bg-[var(--text-main)]/10 group-hover:text-[var(--text-main)] border border-[var(--glass-border)]'}"
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
                  >
                    <path d={prayerIcons[prayer]} />
                  </svg>
                </div>
                <div>
                  <div class="flex items-center gap-2 sm:gap-3">
                    <p
                      class="font-bold capitalize text-[clamp(0.85rem,3vh,1.1rem)] md:text-[clamp(1rem,3vh,1.25rem)] tracking-wide {isCurrent
                        ? 'text-[var(--text-main)] drop-shadow-md'
                        : 'text-[var(--text-main)]/80'}"
                    >
                      {prayer}
                    </p>
                    {#if isCurrent}
                      <span
                        class="text-[10px] font-bold uppercase tracking-wider bg-emerald-500/20 px-2 py-0.5 rounded-full ring-1 ring-emerald-500/50 text-emerald-300 shadow-[0_0_10px_rgba(16,185,129,0.3)]"
                        >Now</span
                      >
                    {:else if isNext}
                      <span
                        class="text-[10px] font-bold uppercase tracking-wider bg-indigo-500/30 px-2 py-0.5 rounded-full text-indigo-200 ring-1 ring-indigo-500/50"
                        >Next</span
                      >
                    {/if}
                  </div>
                </div>
              </div>
              <span
                class="text-[clamp(0.85rem,3vh,1.1rem)] sm:text-lg font-mono tracking-wider relative z-10 {isCurrent
                  ? 'font-black text-[var(--text-main)]'
                  : 'font-medium text-[var(--text-main)]/70'}"
                >{formatPrayerTime(time)}</span
              >
            </div>
          {/each}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  /* Font for Arabic */
  .font-arabic {
    font-family: "Segoe UI", Tahoma, Geneva, Verdana, "Amiri",
      "Traditional Arabic", serif;
  }

  @media (max-height: 800px) {
    .praytime-page {
      gap: 0.5rem;
      padding-bottom: 0.5rem;
    }

    .city-title {
      font-size: clamp(1.8rem, 5vw, 2.8rem);
      line-height: 1.02;
    }

    .countdown-card {
      min-height: 0;
      padding: 0.75rem;
    }

    .ring-visual {
      width: min(15rem, 40vh);
      height: min(15rem, 40vh);
    }

    .minutes-text {
      font-size: clamp(2rem, 7vh, 3.5rem);
    }
  }
</style>
