<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "$lib/desktop/api";
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
  let isVisible = $state(true);
  let timeString = $state("");
  let currentDate = $state(new Date());
  let isFetching = false;
  let lastPrayer = $state<string | null>(null);
  let rotationDegrees = $state(0);

  // Helper to convert HH:MM to minutes from midnight
  function timeStringToMinutes(timeStr: string): number {
    if (!timeStr || timeStr === "--:--") return 0;
    const [h, m] = timeStr.split(":").map(Number);
    return h * 60 + m;
  }

  // Hourglass sand computations - cached and only recalculated when prayer times change
  let hourglassProgress = $derived.by(() => {
    // Only recalculate when prayerTimes reference changes, not on every tick
    const pt = prayerTimes;
    if (!pt || pt.minutes_to_next === null || pt.minutes_to_next === undefined) {
      return 0;
    }

    if (!pt.current_prayer || !pt.next_prayer) {
      return Math.max(0, Math.min(1, getProgress(pt.minutes_to_next) / 100));
    }

    // Attempt to calculate total time between current (previous) and next prayer
    const currentKey = pt.current_prayer.toLowerCase() as keyof PrayerTimes;
    const nextKey = pt.next_prayer.toLowerCase() as keyof PrayerTimes;

    const currentTimeStr = pt[currentKey] as string;
    const nextTimeStr = pt[nextKey] as string;

    if (!currentTimeStr || !nextTimeStr || currentTimeStr === "--:--" || nextTimeStr === "--:--") {
      return Math.max(0, Math.min(1, getProgress(pt.minutes_to_next) / 100));
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
    const elapsedMinutes = totalMinutes - pt.minutes_to_next;
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

  let hijriOffset = $state(0);

  async function loadData() {
    const isDesktop =
      typeof window !== "undefined" &&
      ("__TAURI_INTERNALS__" in window ||
        "electronAPI" in window);

    if (!isDesktop) {
      error =
        "Desktop runtime not available. Please run the desktop application or use `npm run tauri:dev` for Tauri.";
      loading = false;
      return;
    }

    if (isFetching) return;
    isFetching = true;

    try {
      // Fetch Prayer Times from Rust Backend
      const times = await invoke<PrayerTimes>("get_prayer_times", {});
      prayerTimes = times;
      $currentPrayer = times.next_prayer;

      // Try fetching Aladhan Hijri Date online, applying the user's offset
      let hijriDateData: HijriDate | null = null;
      try {
        // Apply the offset to the date we look up, so Aladhan returns the right day
        const lookupDate = new Date();
        lookupDate.setDate(lookupDate.getDate() + hijriOffset);
        const dd = String(lookupDate.getDate()).padStart(2, "0");
        const mm = String(lookupDate.getMonth() + 1).padStart(2, "0");
        const yyyy = lookupDate.getFullYear();

        const response = await fetch(
          `http://api.aladhan.com/v1/gToH/${dd}-${mm}-${yyyy}`,
        );
        if (response.ok) {
          const data = await response.json();
          const hd = data.data.hijri;
          hijriDateData = {
            year: parseInt(hd.year),
            month: hd.month.number,
            day: parseInt(hd.day),
            month_name: hd.month.en,
            formatted: `${parseInt(hd.day)} ${hd.month.en} ${hd.year}`,
            formatted_arabic: `${hd.day} ${hd.month.ar} ${hd.year}`,
          };
        }
      } catch (err) {
        console.warn(
          "Could not fetch online hijri date, falling back to local computation:",
          err,
        );
      }

      // Fallback to local Rust computation, passing the offset
      if (!hijriDateData) {
        hijriDateData = await invoke<HijriDate>("get_hijri_date", {
          offsetDays: hijriOffset,
        });
      }

      hijriDate = hijriDateData;
      error = null;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
      isFetching = false;
    }
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

  function updateTime() {
    const now = new Date();
    timeString = now.toLocaleTimeString("en-US", {
      hour: "2-digit",
      minute: "2-digit",
      hour12: $clockFormat !== "hour24",
    });
    currentDate = now;
  }

  onMount(() => {
    // Load config first so hijriOffset and clockFormat are set before loadData runs
    invoke("get_config")
      .then((cfg: any) => {
        if (cfg?.appearance?.clock_format) {
          $clockFormat = cfg.appearance.clock_format;
        }
        if (typeof cfg?.hijri_offset === "number") {
          hijriOffset = cfg.hijri_offset;
        }
      })
      .catch(() => {})
      .finally(() => {
        loadData();
      });

    // Initial time update
    updateTime();

    // Update time every second - only update string, not full Date
    const interval = setInterval(() => {
      if (isVisible) {
        updateTime();
      }
    }, 1000);

    // Refresh prayer times every minute
    const refreshInterval = setInterval(() => {
      if (isVisible) {
        loadData();
      }
    }, 60000);

    // Handle visibility changes
    const handleVisibilityChange = () => {
      isVisible = document.visibilityState === "visible";
      if (isVisible) {
        // Immediate update when becoming visible
        updateTime();
        loadData();
      }
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);

    return () => {
      clearInterval(interval);
      clearInterval(refreshInterval);
      document.removeEventListener("visibilitychange", handleVisibilityChange);
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
    aria-live="polite"
    role="status"
  >
    <div class="glass-card p-8 rounded-2xl">
      <p class="text-center text-lg text-[var(--text-main)]">
        Loading prayer times…
      </p>
    </div>
  </div>
{:else if error}
  <div
    class="flex items-center justify-center h-full min-h-[220px]"
    transition:fade
    aria-live="polite"
    role="alert"
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
    class="praytime-page flex flex-col gap-[clamp(0.25rem,1.5vmin,0.5rem)] h-full min-h-0 max-w-6xl mx-auto w-full pt-[clamp(0.25rem,1vmin,0.5rem)] pb-[clamp(0.25rem,1.5vmin,0.5rem)] overflow-hidden"
    in:fade={{ duration: 500 }}
  >
    <!-- Header -->
    <header
      class="flex flex-col md:flex-row justify-between items-end gap-[clamp(0.5rem,1.5vmin,1.25rem)] px-[clamp(0.75rem,3vmin,2rem)] relative mb-[clamp(0.25rem,1.5vmin,0.5rem)] flex-shrink-0"
    >
      <div class="z-10 min-w-0 flex-1 w-full text-center md:text-left">
        <h1
          class="city-title text-[clamp(1.5rem,6vmin,3rem)] font-black tracking-tighter text-[var(--text-main)] drop-shadow-sm truncate"
          title={prayerTimes?.location_name || "Location"}
        >
          {prayerTimes?.location_name || "Location"}
        </h1>
        <p
          class="text-[var(--text-muted)] mt-[clamp(0.125rem,0.5vmin,0.25rem)] font-medium tracking-wider uppercase text-[clamp(0.6rem,1.8vmin,0.75rem)]"
        >
          {currentDate.toLocaleDateString("en-US", {
            weekday: "long",
            year: "numeric",
            month: "long",
            day: "numeric",
          })}
        </p>
      </div>
      <div
        class="text-center md:text-right w-full md:w-auto z-10 hidden sm:block self-start lg:self-auto"
      >
        <p
          class="text-[clamp(1rem,3vmin,1.5rem)] font-bold tracking-tight text-[var(--text-main)]/90"
        >
          {hijriDate?.formatted}
        </p>
        <p
          class="text-[var(--text-muted)] font-arabic text-[clamp(0.875rem,2.5vmin,1.25rem)] mt-[clamp(0.125rem,0.5vmin,0.25rem)]"
        >
          {hijriDate?.formatted_arabic}
        </p>
      </div>
    </header>

    <div
      class="grid grid-cols-1 lg:grid-cols-12 gap-[clamp(0.5rem,2vmin,1rem)] flex-1 min-h-0"
    >
      <!-- Left Column: Timer & Info -->
      <div
        class="flex flex-col gap-[clamp(0.5rem,1.5vmin,0.75rem)] lg:col-span-6 xl:col-span-7 h-full min-h-0"
      >
        <!-- Countdown Card -->
        <div
          class="countdown-card backdrop-blur-xl bg-[var(--glass-bg)] border border-[var(--glass-border)] p-[clamp(0.5rem,3vmin,1.25rem)] rounded-[clamp(1rem,4vmin,2rem)] flex flex-col items-center justify-center relative overflow-hidden flex-1 min-h-0 shadow-[0_8px_32px_var(--glass-shadow)] group"
        >
          <div
            class="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-[150%] md:w-[120%] aspect-square bg-[radial-gradient(circle,rgba(255,255,255,0.1)_0%,transparent_70%)] opacity-50 group-hover:opacity-100 transition-opacity duration-1000 pointer-events-none clamp-size"
          ></div>

          <div
            class="relative z-10 text-center flex flex-col items-center w-full"
          >
            <div
              class="bg-[var(--glass-bg)] border border-[var(--glass-border)] px-[clamp(0.75rem,4vmin,1.5rem)] py-[clamp(0.25rem,1.5vmin,0.5rem)] rounded-[clamp(1rem,4vmin,2rem)] backdrop-blur-md mb-[clamp(0.25rem,1.5vmin,0.5rem)] shadow-inner"
            >
              <span
                class="text-[clamp(0.75rem,3vmin,1.125rem)] font-mono font-medium tracking-widest text-[var(--text-main)]/90 drop-shadow-[0_0_8px_rgba(255,255,255,0.1)]"
              >
                {timeString}
              </span>
            </div>

            {#if prayerTimes?.next_prayer && prayerTimes?.minutes_to_next !== null}
              <div
                class="relative flex items-center justify-center min-h-0 flex-1 max-h-[40vh]"
                role="timer"
                aria-label="{prayerTimes.minutes_to_next} minutes until {prayerTimes.next_prayer}"
              >
                <!-- Enhanced Progress Ring / Circular Hourglass -->
                <svg
                  class="ring-visual w-[clamp(8rem,45vmin,18rem)] max-w-full aspect-square drop-shadow-[0_0_30px_rgba(96,165,250,0.15)] p-[clamp(0.25rem,1vmin,0.5rem)] transition-transform duration-1000 ease-in-out"
                  style="transform: rotate({rotationDegrees}deg) scale(1);"
                  viewBox="0 0 288 288"
                  aria-hidden="true"
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
                  aria-hidden="true"
                >
                  <span
                    class="minutes-text text-[clamp(1.5rem,8vmin,3.5rem)] font-black tracking-tighter text-[var(--text-main)] drop-shadow-lg leading-none"
                    >{formatMinutes(prayerTimes.minutes_to_next)}</span
                  >
                  <span
                    class="text-[var(--text-muted)] text-[clamp(0.5rem,1.8vmin,0.75rem)] uppercase tracking-[0.2em] mt-[clamp(0.125rem,0.5vmin,0.25rem)] font-bold"
                    >Until {prayerTimes.next_prayer}</span
                  >
                </div>
              </div>

              <div
                class="mt-[clamp(0.25rem,1.5vh,0.5rem)] flex items-center justify-center gap-[clamp(0.25rem,1vh,0.5rem)] bg-[var(--glass-bg)] px-[clamp(0.5rem,3vh,0.75rem)] py-[clamp(0.25rem,1.2vh,0.5rem)] rounded-[clamp(0.5rem,2vh,0.75rem)] border border-[var(--glass-border)] backdrop-blur-md w-full max-w-[14rem] shadow-inner flex-shrink-0"
              >
                <div
                  class="w-[clamp(0.4rem,1.5vh,0.6rem)] h-[clamp(0.4rem,1.5vh,0.6rem)] rounded-full {prayerTimes.current_prayer
                    ? 'bg-emerald-400 animate-pulse shadow-[0_0_10px_rgba(52,211,153,0.8)]'
                    : 'bg-[var(--text-main)]/30'}"
                ></div>
                <p
                  class="text-[var(--text-muted)] font-medium tracking-wide text-[clamp(0.7rem,2.2vh,0.875rem)] w-full text-center md:text-left"
                >
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
        class="daily-card backdrop-blur-2xl bg-[var(--glass-bg)] border border-[var(--glass-border)] p-[clamp(0.5rem,2vh,1rem)] rounded-[clamp(1rem,4vh,2rem)] flex flex-col lg:col-span-6 xl:col-span-5 relative overflow-hidden shadow-[0_8px_32px_var(--glass-shadow)] flex-1 min-h-[40vh]"
      >
        <div
          class="absolute right-0 top-0 w-[120%] lg:w-[100%] aspect-square bg-[radial-gradient(circle,rgba(255,255,255,0.05)_0%,transparent_70%)] pointer-events-none -translate-y-1/2"
        ></div>

        <h2
          class="text-[clamp(0.875rem,3vh,1.125rem)] font-bold tracking-tight mb-[clamp(0.125rem,1vh,0.5rem)] px-[clamp(0.25rem,1vh,0.5rem)] text-[var(--text-main)]/90 flex-shrink-0"
        >
          Daily Times
        </h2>

        <div
          class="relative z-10 flex-1 flex flex-col justify-between gap-[clamp(0.125rem,1vh,0.5rem)] overflow-hidden"
          role="list"
          aria-label="Daily prayer times"
        >
          {#each prayers as prayer, i}
            {@const time = prayerTimes?.[prayer] || "--:--"}
            {@const isCurrent =
              prayerTimes?.current_prayer?.toLowerCase() === prayer}
            {@const isNext = prayerTimes?.next_prayer?.toLowerCase() === prayer}

            <div
              class="flex items-center justify-between p-[clamp(0.25rem,1.5vh,0.75rem)] rounded-[clamp(0.5rem,2vh,0.75rem)] transition-all duration-500 ease-out group relative overflow-hidden flex-1 min-h-0
                 {isCurrent
                ? 'bg-[var(--text-main)]/10 shadow-[0_4px_24px_var(--glass-shadow)] border border-[var(--glass-border)] scale-[1.02]'
                : 'bg-[var(--text-main)]/5 border border-transparent hover:bg-[var(--text-main)]/10 hover:border-[var(--glass-border)]'} 
                 {isNext ? 'bg-blue-500/10 border-blue-500/30' : ''}"
              style="transition-delay: {i * 40}ms"
              in:fly={{ x: 20, duration: 600, delay: i * 80 }}
              role="listitem"
            >
              <!-- Inner glow for current prayer -->
              {#if isCurrent}
                <div
                  class="absolute inset-0 bg-gradient-to-r from-white/10 to-transparent opacity-50 pointer-events-none"
                ></div>
              {/if}

              <div
                class="flex items-center gap-[clamp(0.5rem,2.5vh,1.25rem)] relative z-10 min-w-0"
              >
                <div
                  class="w-[clamp(1.5rem,5.5vh,2.5rem)] h-[clamp(1.5rem,5.5vh,2.5rem)] flex items-center justify-center rounded-lg transition-colors duration-300
                    {isCurrent
                    ? 'bg-[var(--text-main)] text-[var(--on-text-main)] shadow-lg shadow-[var(--text-main)]/20'
                    : 'bg-[var(--text-main)]/5 text-[var(--text-main)]/60 group-hover:bg-[var(--text-main)]/10 group-hover:text-[var(--text-main)] border border-[var(--glass-border)]'}"
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width="100%"
                    height="100%"
                    class="w-[50%] h-[50%]"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    stroke-width="2"
                    stroke-linecap="round"
                    stroke-linejoin="round"
                    aria-hidden="true"
                  >
                    <path d={prayerIcons[prayer]} />
                  </svg>
                </div>
                <div class="min-w-0 flex items-center justify-center">
                  <div
                    class="flex items-center gap-[clamp(0.25rem,1vh,0.75rem)]"
                  >
                    <p
                      class="font-bold capitalize text-[clamp(0.75rem,2.8vh,1.1rem)] tracking-wide truncate {isCurrent
                        ? 'text-[var(--text-main)] drop-shadow-md'
                        : 'text-[var(--text-main)]/80'}"
                    >
                      {prayer}
                    </p>
                    {#if isCurrent}
                      <span
                        class="text-[clamp(0.4rem,1.2vh,0.625rem)] font-bold uppercase tracking-wider bg-emerald-500/20 px-[clamp(0.2rem,1vh,0.5rem)] py-[clamp(0.125rem,0.5vh,0.125rem)] rounded-full ring-1 ring-emerald-500/50 text-emerald-300 shadow-[0_0_10px_rgba(16,185,129,0.3)]"
                        >Now</span
                      >
                    {:else if isNext}
                      <span
                        class="text-[clamp(0.4rem,1.2vh,0.625rem)] font-bold uppercase tracking-wider bg-indigo-500/30 px-[clamp(0.2rem,1vh,0.5rem)] py-[clamp(0.125rem,0.5vh,0.125rem)] rounded-full text-indigo-200 ring-1 ring-indigo-500/50"
                        >Next</span
                      >
                    {/if}
                  </div>
                </div>
              </div>
              <span
                class="text-[clamp(0.7rem,2.8vh,1rem)] font-mono tracking-wider relative z-10 whitespace-nowrap {isCurrent
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
</style>
