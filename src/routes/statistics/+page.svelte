<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { fade, slide } from "svelte/transition";

  interface StatisticsData {
    total_prayers_logged: number;
    completion_rate_percentage: number;
    current_streak: number;
    longest_streak: number;
    per_prayer_completion: Record<string, number>;
  }

  let stats = $state<StatisticsData | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  const prayers = ["Fajr", "Sunrise", "Dhuhr", "Asr", "Maghrib", "Isha"];

  const prayerColors: Record<string, string> = {
    Fajr: "#818cf8",
    Sunrise: "#fbbf24",
    Dhuhr: "#38bdf8",
    Asr: "#a78bfa",
    Maghrib: "#f87171",
    Isha: "#60a5fa",
  };

  async function loadStats() {
    loading = true;
    error = null;
    try {
      // Try backend first
      try {
        stats = await invoke<StatisticsData>("get_statistics");
      } catch {
        // Backend not implemented yet — start with zero data
        stats = {
          total_prayers_logged: 0,
          completion_rate_percentage: 0,
          current_streak: 0,
          longest_streak: 0,
          per_prayer_completion: {
            Fajr: 0,
            Sunrise: 0,
            Dhuhr: 0,
            Asr: 0,
            Maghrib: 0,
            Isha: 0,
          },
        };
      }
    } catch (e) {
      console.error(e);
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    loadStats();
  });
</script>

<svelte:head>
  <title>Statistics - NanoPrayer</title>
</svelte:head>

<div
  class="h-full min-h-0 bg-transparent text-[var(--text-main)] p-3 sm:p-4 md:p-6 flex flex-col pt-1 md:pt-2 overflow-hidden"
  in:fade={{ duration: 400 }}
>
  <div
    class="max-w-5xl mx-auto w-full flex-1 flex flex-col gap-6 overflow-y-auto pr-2"
  >
    <!-- Header -->
    <div
      class="backdrop-blur-xl bg-[var(--glass-bg)] border border-[var(--glass-border)] p-6 rounded-2xl flex flex-col md:flex-row justify-between items-start md:items-center relative overflow-hidden gap-4 shadow-[0_8px_32px_var(--glass-shadow)] flex-shrink-0"
    >
      <div
        class="absolute -right-20 -top-20 w-64 h-64 bg-blue-500/10 blur-3xl rounded-full pointer-events-none"
      ></div>

      <div class="relative z-10">
        <h2
          class="text-3xl md:text-4xl font-black tracking-tighter drop-shadow-md text-[var(--text-main)] mb-1"
        >
          Prayer Analytics
        </h2>
        <p
          class="text-[var(--text-muted)] font-medium tracking-wide uppercase text-xs"
        >
          Track your consistency and spiritual goals
        </p>
      </div>
    </div>

    {#if loading}
      <div
        class="flex-1 flex flex-col items-center justify-center min-h-[300px]"
      >
        <div
          class="w-12 h-12 rounded-full border-4 border-[var(--primary)]/20 border-t-[var(--primary)] animate-spin mb-4"
        ></div>
        <p class="text-[var(--text-muted)] font-medium">
          Gathering statistics...
        </p>
      </div>
    {:else if error}
      <div
        class="glass-card p-8 rounded-2xl border border-red-500/30 flex flex-col items-center justify-center flex-1 bg-red-500/5"
      >
        <p class="text-red-200 font-medium mb-4 text-xl">
          Could not load statistics
        </p>
        <p class="text-[var(--text-muted)] mb-8 max-w-md text-center">{error}</p>
        <button type="button" onclick={loadStats} class="action-btn">Retry</button>
      </div>
    {:else if stats}
      <!-- Top Overview Stats -->
      <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <!-- Completion Rate -->
        <div
          class="backdrop-blur-2xl bg-[var(--glass-bg)] border border-[var(--glass-border)] p-6 rounded-3xl relative overflow-hidden group shadow-[0_8px_32px_var(--glass-shadow)] flex flex-col justify-between"
        >
          <div
            class="absolute inset-0 bg-gradient-to-br from-emerald-500/10 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-1000 pointer-events-none mix-blend-screen"
          ></div>
          <div class="flex items-center gap-4 mb-4 relative z-10">
            <div
              class="w-12 h-12 rounded-2xl bg-emerald-500/20 flex items-center justify-center text-emerald-400 border border-emerald-500/20 shadow-[0_0_15px_rgba(16,185,129,0.2)]"
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
                ><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" /><polyline
                  points="22 4 12 14.01 9 11.01"
                /></svg
              >
            </div>
            <h3
              class="text-[var(--text-muted)] font-bold uppercase tracking-wider text-xs"
            >
              Completion
            </h3>
          </div>
          <p
            class="text-4xl font-black tracking-tighter text-[var(--text-main)] drop-shadow-lg relative z-10"
          >
            {stats.completion_rate_percentage}<span
              class="text-xl text-[var(--text-muted)]">%</span
            >
          </p>
        </div>

        <!-- Current Streak -->
        <div
          class="backdrop-blur-2xl bg-[var(--glass-bg)] border border-[var(--glass-border)] p-5 rounded-2xl relative overflow-hidden group shadow-[0_8px_32px_var(--glass-shadow)]"
        >
          <div
            class="absolute inset-0 bg-gradient-to-br from-orange-500/10 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-1000 pointer-events-none mix-blend-screen"
          ></div>
          <div class="flex items-center gap-3 mb-4 relative z-10">
            <div
              class="w-10 h-10 rounded-xl bg-orange-500/20 flex items-center justify-center text-orange-400 border border-orange-500/20 shadow-[0_0_15px_rgba(249,115,22,0.2)]"
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
                ><path
                  d="M8.5 14.5A2.5 2.5 0 0 0 11 12c0-1.38-.5-2-1-3-1.072-2.143-.224-4.054 2-6 .5 2.5 2 4.9 4 6.5 2 1.6 3 3.5 3 5.5a7 7 0 1 1-14 0c0-1.153.433-2.294 1-3a2.5 2.5 0 0 0 2.5 2.5z"
                /></svg
              >
            </div>
            <h3
              class="text-[var(--text-muted)] font-bold uppercase tracking-wider text-xs"
            >
              Active Streak
            </h3>
          </div>
          <div class="flex items-baseline gap-2 relative z-10">
            <p
              class="text-4xl font-black tracking-tighter text-[var(--text-main)] drop-shadow-lg"
            >
              {stats.current_streak}
            </p>
            <span
              class="text-[var(--text-muted)] font-bold uppercase text-xs tracking-widest"
              >Days</span
            >
          </div>
        </div>

        <!-- Longest Streak -->
        <div
          class="backdrop-blur-2xl bg-[var(--glass-bg)] border border-[var(--glass-border)] p-5 rounded-2xl relative overflow-hidden group shadow-[0_8px_32px_var(--glass-shadow)]"
        >
          <div
            class="absolute inset-0 bg-gradient-to-br from-purple-500/10 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-1000 pointer-events-none mix-blend-screen"
          ></div>
          <div class="flex items-center gap-3 mb-4 relative z-10">
            <div
              class="w-10 h-10 rounded-xl bg-purple-500/20 flex items-center justify-center text-purple-400 border border-purple-500/20 shadow-[0_0_15px_rgba(168,85,247,0.2)]"
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
                ><polygon
                  points="12 2 15.09 8.26 22 9.27 17 14.14 18.18 21.02 12 17.77 5.82 21.02 7 14.14 2 9.27 8.91 8.26 12 2"
                /></svg
              >
            </div>
            <h3
              class="text-[var(--text-muted)] font-bold uppercase tracking-wider text-xs"
            >
              Best Streak
            </h3>
          </div>
          <div class="flex items-baseline gap-2 relative z-10">
            <p
              class="text-4xl font-black tracking-tighter text-[var(--text-main)] drop-shadow-lg"
            >
              {stats.longest_streak}
            </p>
            <span
              class="text-[var(--text-muted)] font-bold uppercase text-xs tracking-widest"
              >Days</span
            >
          </div>
        </div>

        <!-- Total Prayers -->
        <div
          class="backdrop-blur-2xl bg-[var(--glass-bg)] border border-[var(--glass-border)] p-5 rounded-2xl relative overflow-hidden group shadow-[0_8px_32px_var(--glass-shadow)]"
        >
          <div
            class="absolute inset-0 bg-gradient-to-br from-blue-500/10 to-transparent opacity-0 group-hover:opacity-100 transition-opacity duration-1000 pointer-events-none mix-blend-screen"
          ></div>
          <div class="flex items-center gap-3 mb-4 relative z-10">
            <div
              class="w-10 h-10 rounded-xl bg-blue-500/20 flex items-center justify-center text-blue-400 border border-blue-500/20 shadow-[0_0_15px_rgba(59,130,246,0.2)]"
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
                ><path
                  d="M4 19.5v-15A2.5 2.5 0 0 1 6.5 2H20v20H6.5a2.5 2.5 0 0 1 0-5H20"
                /></svg
              >
            </div>
            <h3
              class="text-[var(--text-muted)] font-bold uppercase tracking-wider text-xs"
            >
              Total Logged
            </h3>
          </div>
          <p
            class="text-4xl font-black tracking-tighter text-[var(--text-main)] drop-shadow-lg relative z-10"
          >
            {stats.total_prayers_logged}
          </p>
        </div>
      </div>

      <!-- Empty state or breakdown -->
      {#if stats.total_prayers_logged === 0}
        <div
          class="backdrop-blur-xl bg-[var(--glass-bg)] border border-[var(--glass-border)] p-8 rounded-2xl flex flex-col items-center justify-center text-center flex-1 min-h-[200px] shadow-[0_8px_32px_var(--glass-shadow)]"
        >
          <div class="text-5xl mb-4">📿</div>
          <h3 class="text-xl font-bold text-[var(--text-main)] mb-2">
            No prayers logged yet
          </h3>
          <p class="text-[var(--text-muted)] max-w-md text-sm leading-relaxed">
            Your statistics will build up as prayer reminders fire and you
            interact with them. Each prayer you mark as completed contributes to
            your streaks and completion rate.
          </p>
        </div>
      {:else}
        <!-- Prayer Breakdown Chart -->
        <div
          class="backdrop-blur-xl bg-[var(--glass-bg)] border border-[var(--glass-border)] p-6 rounded-2xl flex-1 relative overflow-hidden shadow-[0_8px_32px_var(--glass-shadow)]"
        >
          <div
            class="absolute right-0 bottom-0 w-[500px] h-[500px] bg-[radial-gradient(circle,rgba(79,70,229,0.1)_0%,transparent_60%)] blur-3xl rounded-full pointer-events-none mix-blend-screen"
          ></div>

          <h3
            class="text-xl font-black tracking-tighter mb-6 text-[var(--text-main)]"
          >
            Daily breakdown
          </h3>

          <div class="space-y-5 max-w-4xl relative z-10">
            {#each prayers as prayer}
              {@const percentage = stats.per_prayer_completion[prayer] || 0}
              {@const color = prayerColors[prayer]}

              <div class="relative group">
                <div class="flex justify-between items-end mb-1.5">
                  <span
                    class="font-bold text-[var(--text-main)]/90 tracking-wide"
                    >{prayer}</span
                  >
                  <span
                    class="font-mono font-bold tracking-widest text-sm"
                    style="color: {color}; text-shadow: 0 0 10px {color}80"
                    >{percentage}%</span
                  >
                </div>

                <div
                  class="h-4 w-full bg-[var(--text-main)]/10 rounded-full overflow-hidden shadow-inner backdrop-blur-md border border-[var(--glass-border)]"
                >
                  <div
                    class="h-full rounded-full transition-all duration-1000 ease-out relative overflow-hidden"
                    style="width: {percentage}%; background-color: {color}; box-shadow: 0 0 20px {color}80;"
                    in:slide={{ duration: 1000, axis: "x" }}
                  >
                    <div
                      class="absolute top-0 left-0 w-full h-1/2 bg-gradient-to-b from-white/40 to-transparent"
                    ></div>
                    <div
                      class="absolute inset-0 bg-gradient-to-r from-transparent via-white/20 to-transparent -translate-x-full group-hover:animate-[shimmer_2s_infinite]"
                    ></div>
                  </div>
                </div>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>
