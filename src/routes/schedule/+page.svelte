<script lang="ts">
  import { onMount, tick } from "svelte";
  import { invoke } from "$lib/desktop/api";
  import { fade, fly } from "svelte/transition";

  import { clockFormat } from "$lib/stores";

  interface PrayerTimesResponse {
    date: string;
    fajr: string;
    sunrise: string;
    dhuhr: string;
    asr: string;
    maghrib: string;
    isha: string;
  }

  interface DayWithComputed extends PrayerTimesResponse {
    dayDate: Date;
    isToday: boolean;
    dayNumber: number;
  }

  let currentMonth = $state(new Date());
  let days = $state<DayWithComputed[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let scrollContainer: HTMLDivElement | undefined = $state(undefined);

  async function loadMonth() {
    loading = true;
    error = null;
    try {
      const year = currentMonth.getFullYear();
      const month = currentMonth.getMonth() + 1;
      const response = await invoke<PrayerTimesResponse[]>("get_monthly_prayer_times", { year, month });

      // Pre-compute date values once to avoid creating Date objects during render
      const today = new Date();
      days = response.map((day) => {
        const dayDate = new Date(day.date);
        return {
          ...day,
          dayDate,
          isToday:
            dayDate.getDate() === today.getDate() &&
            dayDate.getMonth() === today.getMonth() &&
            dayDate.getFullYear() === today.getFullYear(),
          dayNumber: dayDate.getDate(),
        };
      });

      // Auto-scroll to today after data loads
      await tick();
      setTimeout(scrollToToday, 100);
    } catch (e) {
      console.error(e);
      error = String(e);
    } finally {
      loading = false;
    }
  }

  function scrollToToday() {
    if (!scrollContainer) return;
    const todayRow = scrollContainer.querySelector('[data-today="true"]');
    if (todayRow) {
      setTimeout(() => {
        todayRow.scrollIntoView({ behavior: "smooth", block: "center" });
      }, 50);
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

  function nextMonth() {
    currentMonth = new Date(
      currentMonth.getFullYear(),
      currentMonth.getMonth() + 1,
      1,
    );
    loadMonth();
  }

  function prevMonth() {
    currentMonth = new Date(
      currentMonth.getFullYear(),
      currentMonth.getMonth() - 1,
      1,
    );
    loadMonth();
  }

  onMount(() => {
    loadMonth();
  });
</script>

<svelte:head>
  <title>Schedule - NanoPrayer</title>
</svelte:head>

<div
  class="h-full min-h-0 bg-transparent text-[var(--text-main)] p-3 sm:p-4 md:p-6 flex flex-col pt-1 md:pt-2 overflow-hidden"
  in:fade={{ duration: 400 }}
>
  <div
    class="max-w-5xl mx-auto w-full flex-1 flex flex-col overflow-hidden gap-3"
  >
    <!-- Header -->
    <div
      class="np-surface bg-[var(--surface-bg)] border border-[var(--glass-border)] p-4 md:p-5 rounded-2xl flex justify-between items-center relative overflow-hidden shadow-[0_8px_32px_var(--glass-shadow)] flex-shrink-0"
    >
      <div
        class="absolute inset-0 bg-gradient-to-r from-blue-500/5 via-purple-500/5 to-transparent pointer-events-none"
      ></div>

      <button
        type="button"
        onclick={prevMonth}
        aria-label="Previous month"
        title="Previous month"
        class="p-2.5 rounded-xl bg-[var(--text-main)]/5 border border-[var(--glass-border)] hover:bg-[var(--text-main)]/10 transition z-10 hover:scale-105 active:scale-95 text-[var(--text-main)] shadow-md group"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.5"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="group-hover:-translate-x-1 transition-transform"
          ><path d="m15 18-6-6 6-6" /></svg
        >
        <span class="sr-only">Previous month</span>
      </button>

      <div class="text-center z-10 flex flex-col">
        <h2
          class="text-2xl md:text-3xl font-black tracking-tight drop-shadow-md text-[var(--text-main)]"
        >
          {currentMonth.toLocaleString("default", {
            month: "long",
            year: "numeric",
          })}
        </h2>
        <span
          class="text-[var(--text-muted)] text-xs font-bold uppercase tracking-[0.3em] mt-1"
          >Monthly Schedule</span
        >
      </div>

      <button
        type="button"
        onclick={nextMonth}
        aria-label="Next month"
        title="Next month"
        class="p-2.5 rounded-xl bg-[var(--glass-bg)] border border-[var(--glass-border)] hover:bg-white/10 transition z-10 hover:scale-105 active:scale-95 text-[var(--text-main)] shadow-md group"
      >
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.5"
          stroke-linecap="round"
          stroke-linejoin="round"
          class="group-hover:translate-x-1 transition-transform"
          ><path d="m9 18 6-6-6-6" /></svg
        >
        <span class="sr-only">Next month</span>
      </button>
    </div>

    {#if loading}
      <div
        class="flex flex-col items-center justify-center flex-1 glass-card rounded-2xl"
        in:fade
      >
        <div
          class="w-12 h-12 rounded-full border-4 border-[var(--primary)]/20 border-t-[var(--primary)] animate-spin mb-4"
        ></div>
        <div class="animate-pulse text-lg font-medium text-[var(--text-muted)]">
          Calculating prayer times...
        </div>
      </div>
    {:else if error}
      <div
        class="glass-card p-8 rounded-2xl border border-red-500/30 flex flex-col items-center justify-center flex-1 bg-red-500/5 relative overflow-hidden"
      >
        <div
          class="absolute top-0 w-full h-1 bg-gradient-to-r from-transparent via-red-500/50 to-transparent"
        ></div>
        <svg
          xmlns="http://www.w3.org/2000/svg"
          width="48"
          height="48"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2"
          class="text-red-400 mb-4"
          stroke-linecap="round"
          stroke-linejoin="round"
          ><circle cx="12" cy="12" r="10" /><line
            x1="12"
            y1="8"
            x2="12"
            y2="12"
          /><line x1="12" y1="16" x2="12.01" y2="16" /></svg
        >
        <p class="text-red-200 font-medium mb-2 text-xl tracking-tight">
          Failed to load schedule
        </p>
        <p
          class="text-[var(--text-muted)] mb-8 font-mono text-sm max-w-md text-center"
        >
          {error}
        </p>
        <button type="button" onclick={loadMonth} class="action-btn gap-2">
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
            ><path d="M3 12a9 9 0 1 0 9-9 9.75 9.75 0 0 0-6.74 2.74L3 8" /><path
              d="M3 3v5h5"
            /></svg
          >
          Try Again
        </button>
      </div>
    {:else if days.length > 0}
      <div
        class="np-surface bg-[var(--surface-bg)] border border-[var(--glass-border)] rounded-2xl overflow-hidden shadow-[0_20px_60px_var(--glass-shadow)] relative flex-1 flex flex-col min-h-0"
      >
        <div
          class="absolute inset-0 bg-[radial-gradient(ellipse_at_top_right,rgba(255,255,255,0.05)_0%,transparent_50%)] pointer-events-none"
        ></div>
        <div
          bind:this={scrollContainer}
          class="overflow-auto relative z-10 flex-1"
        >
          <table class="w-full text-left border-collapse">
            <thead>
              <tr
                class="bg-[var(--surface-bg)] text-[var(--text-muted)] text-[10px] md:text-xs font-bold uppercase tracking-widest sticky top-0 shadow-sm z-20"
              >
                <th
                  class="p-4 font-bold text-center w-[10%] min-w-[3rem] border-b border-[var(--glass-border)] text-sm"
                  >Day</th
                >
                <th
                  class="p-4 font-bold border-b border-[var(--glass-border)] text-sm"
                  >Fajr</th
                >
                <th
                  class="p-4 font-bold hidden md:table-cell border-b border-[var(--glass-border)] text-sm"
                  >Sunrise</th
                >
                <th
                  class="p-4 font-bold border-b border-[var(--glass-border)] text-sm"
                  >Dhuhr</th
                >
                <th
                  class="p-4 font-bold border-b border-[var(--glass-border)] text-sm"
                  >Asr</th
                >
                <th
                  class="p-4 font-bold border-b border-[var(--glass-border)] text-sm"
                  >Maghrib</th
                >
                <th
                  class="p-4 font-bold border-b border-[var(--glass-border)] text-sm"
                  >Isha</th
                >
              </tr>
            </thead>
            <tbody class="divide-y divide-[var(--glass-border)]">
              {#each days as day, i}
                <tr
                  data-today={day.isToday ? "true" : undefined}
                  class="transition-all duration-300 {day.isToday
                    ? 'bg-gradient-to-r from-blue-500/10 to-purple-500/10 today-row'
                    : 'hover:bg-[var(--text-main)]/5'}"
                  in:fly={{
                    y: 20,
                    delay: Math.min(i * 10, 400),
                    duration: 500,
                    opacity: 0,
                  }}
                >
                  <td class="p-3 md:p-4 text-center">
                    <div
                      class="w-10 h-10 mx-auto rounded-xl flex items-center justify-center font-black text-base transition-transform duration-300 {day.isToday
                        ? 'bg-[var(--primary)] text-white shadow-lg shadow-[var(--primary-glow)] scale-110'
                        : 'text-[var(--text-main)] bg-[var(--text-main)]/5 border border-[var(--glass-border)]'}"
                    >
                      {day.dayNumber}
                    </div>
                  </td>
                  <td
                    class="p-3 md:p-4 font-mono text-sm md:text-base tracking-wide {day.isToday
                      ? 'text-[var(--text-main)] font-bold'
                      : 'text-[var(--text-main)]/80'}"
                    >{formatPrayerTime(day.fajr)}</td
                  >
                  <td
                    class="p-3 md:p-4 font-mono text-sm md:text-base tracking-wide hidden md:table-cell {day.isToday
                      ? 'text-[var(--text-main)]/80'
                      : 'text-[var(--text-main)]/40'}"
                    >{formatPrayerTime(day.sunrise)}</td
                  >
                  <td
                    class="p-3 md:p-4 font-mono text-sm md:text-base tracking-wide {day.isToday
                      ? 'text-[var(--text-main)] font-bold'
                      : 'text-[var(--text-main)]/80'}"
                    >{formatPrayerTime(day.dhuhr)}</td
                  >
                  <td
                    class="p-3 md:p-4 font-mono text-sm md:text-base tracking-wide {day.isToday
                      ? 'text-[var(--text-main)] font-bold'
                      : 'text-[var(--text-main)]/80'}"
                    >{formatPrayerTime(day.asr)}</td
                  >
                  <td
                    class="p-3 md:p-4 font-mono text-sm md:text-base tracking-wide {day.isToday
                      ? 'text-[var(--text-main)] font-bold'
                      : 'text-[var(--text-main)]/80'}"
                    >{formatPrayerTime(day.maghrib)}</td
                  >
                  <td
                    class="p-3 md:p-4 font-mono text-sm md:text-base tracking-wide {day.isToday
                      ? 'text-[var(--text-main)] font-bold'
                      : 'text-[var(--text-main)]/80'}"
                    >{formatPrayerTime(day.isha)}</td
                  >
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .today-row {
    box-shadow:
      inset 3px 0 0 0 #60a5fa,
      0 0 15px rgba(96, 165, 250, 0.3);
  }
</style>
