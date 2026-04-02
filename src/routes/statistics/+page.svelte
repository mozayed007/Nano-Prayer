<script lang="ts">
  import { onMount } from "svelte";
  import { invoke, listen, type UnlistenFn } from "$lib/desktop/api";
  import { fade, slide } from "svelte/transition";

  interface TimelinePoint {
    label: string;
    completed_count: number;
    expected_count: number;
    completion_rate_percentage: number;
  }

  interface PeriodStatistics {
    label: string;
    start_date: string;
    end_date: string;
    completed_count: number;
    expected_count: number;
    completion_rate_percentage: number;
    per_prayer_completion: Record<string, number>;
    per_prayer_completed: Record<string, number>;
    per_prayer_expected: Record<string, number>;
    timeline: TimelinePoint[];
  }

  interface StatisticsData {
    today: PeriodStatistics;
    week: PeriodStatistics;
    month: PeriodStatistics;
    year: PeriodStatistics;
    all_time: PeriodStatistics;
    total_prayers_logged: number;
    current_streak: number;
    longest_streak: number;
  }

  type PeriodKey = "today" | "week" | "month" | "year";

  let stats = $state<StatisticsData | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);
  let selectedPeriod = $state<PeriodKey>("week");

  const prayers = ["Fajr", "Sunrise", "Dhuhr", "Asr", "Maghrib", "Isha"];

  const prayerColors: Record<string, string> = {
    Fajr: "#818cf8",
    Sunrise: "#fbbf24",
    Dhuhr: "#38bdf8",
    Asr: "#a78bfa",
    Maghrib: "#f87171",
    Isha: "#60a5fa",
  };

  const periodLabels: Record<PeriodKey, string> = {
    today: "Daily",
    week: "Weekly",
    month: "Monthly",
    year: "Yearly",
  };

  function getSelectedPeriod(): PeriodStatistics | null {
    if (!stats) return null;
    return stats[selectedPeriod];
  }

  function formatRate(value: number): string {
    return `${Math.round(value * 10) / 10}%`;
  }

  function formatRange(period: PeriodStatistics): string {
    if (period.start_date === period.end_date) return period.start_date;
    return `${period.start_date} to ${period.end_date}`;
  }

  async function loadStats() {
    loading = true;
    error = null;
    try {
      stats = await invoke<StatisticsData>("get_statistics");
    } catch (e) {
      console.error(e);
      error = String(e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    let unlisten: UnlistenFn | undefined;

    void loadStats();
    void listen("statistics-updated", () => {
      void loadStats();
    }).then((fn) => {
      unlisten = fn;
    });

    return () => {
      if (unlisten) unlisten();
    };
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
    class="max-w-6xl mx-auto w-full flex-1 flex flex-col gap-6 overflow-y-auto pr-2"
  >
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
          Daily, weekly, monthly, and yearly completion views
        </p>
      </div>
    </div>

    {#if loading}
      <div class="flex-1 flex flex-col items-center justify-center min-h-[300px]">
        <div
          class="w-12 h-12 rounded-full border-4 border-[var(--primary)]/20 border-t-[var(--primary)] animate-spin mb-4"
        ></div>
        <p class="text-[var(--text-muted)] font-medium">Gathering statistics...</p>
      </div>
    {:else if error}
      <div
        class="glass-card p-8 rounded-2xl border border-red-500/30 flex flex-col items-center justify-center flex-1 bg-red-500/5"
      >
        <p class="text-red-200 font-medium mb-4 text-xl">Could not load statistics</p>
        <p class="text-[var(--text-muted)] mb-8 max-w-md text-center">{error}</p>
        <button type="button" onclick={loadStats} class="action-btn">Retry</button>
      </div>
    {:else if stats}
      <div class="grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4">
        {#each (Object.keys(periodLabels) as PeriodKey[]) as key}
          {@const period = stats[key]}
          <button
            type="button"
            class="text-left backdrop-blur-2xl bg-[var(--glass-bg)] border rounded-3xl p-5 relative overflow-hidden shadow-[0_8px_32px_var(--glass-shadow)] transition-all {selectedPeriod ===
            key
              ? 'border-blue-500/50 bg-blue-500/10 shadow-[0_0_0_1px_rgba(59,130,246,0.3),0_18px_40px_rgba(37,99,235,0.15)]'
              : 'border-[var(--glass-border)] hover:bg-[var(--text-main)]/6'}"
            onclick={() => (selectedPeriod = key)}
          >
            <div class="flex items-center justify-between gap-4 mb-4">
              <div>
                <div class="text-[var(--text-muted)] font-bold uppercase tracking-wider text-xs mb-1">
                  {periodLabels[key]}
                </div>
                <div class="text-xs text-[var(--text-muted)]/80">{formatRange(period)}</div>
              </div>
              <div
                class="px-2.5 py-1 rounded-full text-[11px] font-bold tracking-wider {selectedPeriod ===
                key
                  ? 'bg-blue-500/20 text-blue-200 border border-blue-400/30'
                  : 'bg-[var(--text-main)]/8 text-[var(--text-muted)] border border-[var(--glass-border)]'}"
              >
                {period.completed_count}/{period.expected_count}
              </div>
            </div>
            <div class="text-4xl font-black tracking-tighter mb-2">
              {formatRate(period.completion_rate_percentage)}
            </div>
            <div class="h-2.5 rounded-full bg-[var(--text-main)]/10 overflow-hidden border border-[var(--glass-border)]">
              <div
                class="h-full rounded-full bg-gradient-to-r from-blue-400 to-indigo-500"
                style={`width: ${Math.max(0, Math.min(period.completion_rate_percentage, 100))}%`}
              ></div>
            </div>
          </button>
        {/each}
      </div>

      <div class="grid grid-cols-2 lg:grid-cols-4 gap-4">
        <div class="stats-mini-card">
          <div class="stats-mini-label">Current streak</div>
          <div class="stats-mini-value">{stats.current_streak}</div>
          <div class="stats-mini-meta">full days in a row</div>
        </div>
        <div class="stats-mini-card">
          <div class="stats-mini-label">Best streak</div>
          <div class="stats-mini-value">{stats.longest_streak}</div>
          <div class="stats-mini-meta">best full-day run</div>
        </div>
        <div class="stats-mini-card">
          <div class="stats-mini-label">Total logged</div>
          <div class="stats-mini-value">{stats.total_prayers_logged}</div>
          <div class="stats-mini-meta">completed prayer entries</div>
        </div>
        <div class="stats-mini-card">
          <div class="stats-mini-label">All-time completion</div>
          <div class="stats-mini-value">{formatRate(stats.all_time.completion_rate_percentage)}</div>
          <div class="stats-mini-meta">{stats.all_time.completed_count}/{stats.all_time.expected_count} due prayers</div>
        </div>
      </div>

      {#if getSelectedPeriod()}
        {@const active = getSelectedPeriod()!}

        <div
          class="backdrop-blur-xl bg-[var(--glass-bg)] border border-[var(--glass-border)] p-6 rounded-2xl shadow-[0_8px_32px_var(--glass-shadow)]"
        >
          <div class="flex flex-col lg:flex-row lg:items-end lg:justify-between gap-4 mb-6">
            <div>
              <h3 class="text-2xl font-black tracking-tighter text-[var(--text-main)]">
                {active.label} view
              </h3>
              <p class="text-[var(--text-muted)] text-sm mt-1">
                {formatRange(active)} · {active.completed_count} completed out of {active.expected_count} due prayers
              </p>
            </div>
            <div class="flex items-center gap-3">
              <div class="px-3 py-2 rounded-xl bg-[var(--text-main)]/7 border border-[var(--glass-border)]">
                <div class="text-[10px] uppercase tracking-[0.18em] text-[var(--text-muted)] font-bold mb-1">
                  Completion
                </div>
                <div class="text-xl font-black tracking-tight">{formatRate(active.completion_rate_percentage)}</div>
              </div>
            </div>
          </div>

          <div class="grid grid-cols-1 xl:grid-cols-[1.2fr_0.8fr] gap-6">
            <div class="stats-panel">
              <div class="stats-panel-title">Trend</div>
              {#if active.timeline.length === 0}
                <div class="stats-empty">No data for this period yet.</div>
              {:else}
                <div class="space-y-3">
                  {#each active.timeline as point}
                    <div>
                      <div class="flex justify-between items-end mb-1.5 gap-3">
                        <span class="text-sm font-semibold text-[var(--text-main)]/90 truncate">
                          {point.label}
                        </span>
                        <span class="text-xs font-mono text-[var(--text-muted)] whitespace-nowrap">
                          {point.completed_count}/{point.expected_count} · {formatRate(point.completion_rate_percentage)}
                        </span>
                      </div>
                      <div class="h-3 w-full bg-[var(--text-main)]/10 rounded-full overflow-hidden border border-[var(--glass-border)]">
                        <div
                          class="h-full rounded-full bg-gradient-to-r from-cyan-400 to-blue-500 transition-all duration-1000"
                          style={`width: ${Math.max(0, Math.min(point.completion_rate_percentage, 100))}%`}
                          in:slide={{ duration: 700, axis: "x" }}
                        ></div>
                      </div>
                    </div>
                  {/each}
                </div>
              {/if}
            </div>

            <div class="stats-panel">
              <div class="stats-panel-title">Per prayer</div>
              <div class="space-y-4">
                {#each prayers as prayer}
                  {@const percentage = active.per_prayer_completion[prayer] || 0}
                  {@const completed = active.per_prayer_completed[prayer] || 0}
                  {@const expected = active.per_prayer_expected[prayer] || 0}
                  {@const color = prayerColors[prayer]}
                  <div class="relative group">
                    <div class="flex justify-between items-end mb-1.5 gap-3">
                      <span class="font-bold text-[var(--text-main)]/90 tracking-wide">{prayer}</span>
                      <span
                        class="font-mono font-bold tracking-widest text-sm whitespace-nowrap"
                        style={`color: ${color}; text-shadow: 0 0 10px ${color}80`}
                      >
                        {completed}/{expected} · {formatRate(percentage)}
                      </span>
                    </div>

                    <div class="h-4 w-full bg-[var(--text-main)]/10 rounded-full overflow-hidden shadow-inner border border-[var(--glass-border)]">
                      <div
                        class="h-full rounded-full transition-all duration-1000 ease-out relative overflow-hidden"
                        style={`width: ${Math.max(0, Math.min(percentage, 100))}%; background-color: ${color}; box-shadow: 0 0 20px ${color}80;`}
                        in:slide={{ duration: 800, axis: "x" }}
                      >
                        <div class="absolute top-0 left-0 w-full h-1/2 bg-gradient-to-b from-white/35 to-transparent"></div>
                      </div>
                    </div>
                  </div>
                {/each}
              </div>
            </div>
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .stats-mini-card {
    backdrop-filter: blur(var(--glass-blur));
    background: var(--glass-bg);
    border: 1px solid var(--glass-border);
    border-radius: 1.25rem;
    padding: 1rem 1.1rem;
    box-shadow: 0 8px 32px var(--glass-shadow);
  }

  .stats-mini-label {
    color: var(--text-muted);
    font-size: 0.72rem;
    font-weight: 800;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    margin-bottom: 0.55rem;
  }

  .stats-mini-value {
    color: var(--text-main);
    font-size: 2rem;
    font-weight: 900;
    line-height: 1;
    letter-spacing: -0.04em;
    margin-bottom: 0.35rem;
  }

  .stats-mini-meta {
    color: var(--text-muted);
    font-size: 0.78rem;
  }

  .stats-panel {
    background: color-mix(in srgb, var(--text-main) 5%, transparent);
    border: 1px solid var(--glass-border);
    border-radius: 1.2rem;
    padding: 1rem;
  }

  .stats-panel-title {
    color: var(--text-main);
    font-size: 1rem;
    font-weight: 800;
    letter-spacing: -0.02em;
    margin-bottom: 1rem;
  }

  .stats-empty {
    color: var(--text-muted);
    font-size: 0.92rem;
    padding: 1rem 0;
  }
</style>
