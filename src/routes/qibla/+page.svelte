<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { fade } from "svelte/transition";

  interface QiblaResponse {
    degrees: number;
    cardinal: string;
    distance_km: number;
  }

  interface Location {
    latitude: number;
    longitude: number;
  }

  let qiblaData = $state<QiblaResponse | null>(null);
  let userLocation = $state<Location | null>(null);
  let deviceHeading = $state(0);
  let loading = $state(true);
  let error = $state<string | null>(null);

  // For compass rotation
  let compassRotation = $derived(deviceHeading * -1);
  let qiblaRotation = $derived(
    qiblaData ? qiblaData.degrees - deviceHeading : 0,
  );

  // Smoothing for compass
  let currentCompassRotation = $state(0);
  let currentQiblaRotation = $state(0);
  let animationFrame: number;

  function lerp(start: number, end: number, factor: number) {
    // Handle shortest path for rotation
    let difference = end - start;
    while (difference < -180) difference += 360;
    while (difference > 180) difference -= 360;
    return start + difference * factor;
  }

  function animate() {
    currentCompassRotation = lerp(currentCompassRotation, compassRotation, 0.1);
    currentQiblaRotation = lerp(currentQiblaRotation, qiblaRotation, 0.1);
    animationFrame = requestAnimationFrame(animate);
  }

  async function initCompass() {
    loading = true;
    error = null;

    try {
      const configRes = await invoke<any>("get_config");
      let lat = 21.4225;
      let lng = 39.8262; // Default to Makkah

      if (configRes?.locations?.length > 0) {
        const activeIndex = Math.min(
          Math.max(configRes.current_location_index ?? 0, 0),
          configRes.locations.length - 1,
        );
        const selectedLocation = configRes.locations[activeIndex];
        if (selectedLocation?.coordinates) {
          lat = selectedLocation.coordinates.latitude;
          lng = selectedLocation.coordinates.longitude;
        }
      }

      userLocation = { latitude: lat, longitude: lng };
      qiblaData = await invoke<QiblaResponse>("get_qibla_direction", {
        latitude: lat,
        longitude: lng,
      });

      // Setup device orientation if available (typically only works on Mobile devices running Tauri)
      if (typeof window !== "undefined" && window.DeviceOrientationEvent) {
        // Request permission for iOS 13+
        if (
          typeof (DeviceOrientationEvent as any).requestPermission ===
          "function"
        ) {
          // We can't request permission automatically without user gesture in standard web,
          // but in Tauri webview it might be allowed depending on the platform constraints.
        }

        window.addEventListener("deviceorientation", handleOrientation);
      }

      loading = false;
      animate();
    } catch (e) {
      console.error(e);
      error = "Failed to load compass data. Ensure your location is set in settings.";
      loading = false;
    }
  }

  function handleOrientation(event: DeviceOrientationEvent) {
    // iOS uses webkitCompassHeading, Android uses alpha
    if (event as any && (event as any).webkitCompassHeading !== undefined) {
      deviceHeading = (event as any).webkitCompassHeading;
    } else if (event.alpha !== null) {
      // Convert alpha to compass heading. Alpha is 0 when device is pointing North on Android,
      // but only if absolute is true.
      if (event.absolute) {
        deviceHeading = 360 - event.alpha;
      }
    }
  }

  onMount(() => {
    initCompass();
  });

  onDestroy(() => {
    if (typeof window !== "undefined") {
      window.removeEventListener("deviceorientation", handleOrientation);
    }
    if (animationFrame) {
      cancelAnimationFrame(animationFrame);
    }
  });
</script>

<svelte:head>
  <title>Qibla - NanoPrayer</title>
</svelte:head>

<div
  class="h-full min-h-0 bg-transparent text-[var(--text-main)] flex flex-col pt-1 md:pt-2 pb-4 md:pb-8 px-3 sm:px-4 overflow-y-auto"
  in:fade={{ duration: 400 }}
>
  {#if loading}
    <div class="flex-1 flex flex-col items-center justify-center">
      <div
        class="w-16 h-16 rounded-full border-4 border-[var(--text-main)]/20 border-t-[var(--text-main)]/80 animate-spin mb-6"
      ></div>
      <p class="text-xl font-medium text-[var(--text-muted)] tracking-wide">
        Calibrating Compass...
      </p>
    </div>
  {:else if error}
    <div class="flex-1 flex flex-col items-center justify-center max-w-lg mx-auto w-full">
      <div class="glass-card p-8 rounded-3xl border border-red-500/30 w-full text-center bg-red-500/5">
        <p class="text-red-200 font-medium mb-4 text-xl tracking-tight">Compass Error</p>
        <p class="text-[var(--text-muted)] mb-8 leading-relaxed max-w-xs mx-auto">
          {error}
        </p>
        <button type="button" onclick={initCompass} class="action-btn"
          >Retry Calibration</button
        >
      </div>
    </div>
  {:else if qiblaData}
    <div class="flex-1 flex flex-col items-center justify-center max-w-2xl mx-auto w-full gap-8 md:gap-12">
      
      <!-- Compass Readout -->
      <div class="text-center">
        <h2 class="text-4xl sm:text-5xl md:text-6xl font-bold tracking-tighter mb-2 text-[var(--text-main)]">
          {qiblaData.degrees.toFixed(1)}°
        </h2>
        <p class="text-[var(--text-muted)] text-xl font-medium tracking-wide uppercase">
          {qiblaData.cardinal}
        </p>
        {#if userLocation}
          <p class="text-[var(--text-muted)]/80 text-xs font-mono mt-2">
            {userLocation.latitude.toFixed(4)},
            {userLocation.longitude.toFixed(4)}
          </p>
        {/if}
      </div>

      <!-- Superior Visual Compass -->
      <div class="relative w-60 h-60 sm:w-72 sm:h-72 md:w-80 md:h-80 flex items-center justify-center perspective-1000">
        <!-- Glow backing -->
        <div class="absolute inset-0 bg-blue-500/10 blur-3xl rounded-full transform scale-150"></div>

        <!-- Outer Track / Bezel -->
        <div
          class="absolute inset-0 rounded-full border border-[var(--glass-border)] bg-[var(--glass-bg)] backdrop-blur-xl shadow-[inset_0_0_50px_rgba(0,0,0,0.35),0_20px_50px_rgba(0,0,0,0.2)]"
        ></div>

        <!-- Degree Ticks (CSS rendering for precision) -->
        <div
          class="absolute inset-2 rounded-full transform-style-3d transition-transform duration-75"
          style="transform: rotate({currentCompassRotation}deg)"
        >
          {#each Array(72) as _, i}
            <div
              class="absolute top-0 left-1/2 w-0.5 h-3 -translate-x-1/2 origin-[50%_152px] {i % 18 === 0 ? 'bg-white/80 h-5 w-1 z-10' : i % 9 === 0 ? 'bg-white/50 h-4' : 'bg-white/20'}"
              style="transform: rotate({i * 5}deg)"
            >
              {#if i === 0}
                <span class="absolute -top-7 -left-2 text-white font-bold text-sm tracking-widest">N</span>
              {:else if i === 18}
                <span class="absolute -top-7 left-1 text-white/50 font-bold text-sm -rotate-90 origin-bottom">E</span>
              {:else if i === 36}
                <span class="absolute -top-7 -left-2 text-white/50 font-bold text-sm -rotate-180">S</span>
              {:else if i === 54}
                <span class="absolute -top-7 -right-3 text-white/50 font-bold text-sm rotate-90 origin-bottom">W</span>
              {/if}
            </div>
          {/each}

          <!-- The Qibla Marker on the Bezel -->
          <div
            class="absolute top-0 left-1/2 -translate-x-1/2 pointer-events-none drop-shadow-[0_0_10px_rgba(59,130,246,0.8)] z-20"
            style="transform: rotate({qiblaData.degrees}deg); transform-origin: 50% 152px;"
          >
            <div class="w-8 h-8 rounded-full bg-blue-500/20 border-2 border-blue-400 flex items-center justify-center backdrop-blur-sm -mt-4 shadow-lg">
              <div class="w-2 h-2 bg-white rounded-full"></div>
            </div>
          </div>
        </div>

        <!-- The Kaaba / Final Destination Inner Pin -->
        <div
          class="absolute w-28 h-28 z-30 flex items-center justify-center transition-transform duration-75"
          style="transform: rotate({currentQiblaRotation}deg)"
        >
          <!-- Needle -->
          <div class="relative w-full h-full">
            <svg viewBox="0 0 100 100" class="w-full h-full drop-shadow-[0_10px_15px_rgba(0,0,0,0.5)]">
              <!-- Shadow -->
              <polygon points="50,10 65,55 50,45" fill="rgba(0,0,0,0.4)" />
              <!-- Right half (Dark) -->
              <polygon points="50,10 65,55 50,90" fill="#2563eb" /> <!-- Blue-600 -->
              <!-- Left half (Light) -->
              <polygon points="50,10 35,55 50,90" fill="#60a5fa" /> <!-- Blue-400 -->
              <!-- Center pivot -->
              <circle cx="50" cy="55" r="5" fill="#1e3a8a" /> <!-- Blue-900 -->
              <circle cx="50" cy="55" r="2" fill="white" />
            </svg>
          </div>
        </div>

        <!-- Center cap cover -->
        <div class="absolute inset-0 m-auto w-16 h-16 rounded-full bg-slate-900 border border-[var(--glass-border)] shadow-[0_5px_15px_rgba(0,0,0,0.5)] z-40 flex items-center justify-center">
          <div class="w-4 h-4 bg-blue-500 rounded-full animate-pulse shadow-[0_0_15px_rgba(59,130,246,0.8)]"></div>
        </div>
      </div>

      <!-- Distance info -->
      <div class="glass-card px-5 sm:px-8 py-3 sm:py-4 rounded-full flex gap-3 items-center mt-2 md:mt-4">
        <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-blue-400"><path d="M20 10c0 6-8 12-8 12s-8-6-8-12a8 8 0 0 1 16 0Z"/><circle cx="12" cy="10" r="3"/></svg>
        <div class="h-4 w-px bg-[var(--glass-border)]"></div>
        <p class="text-[var(--text-main)]/80 font-medium tracking-wide">
          <span class="text-[var(--text-main)] font-bold">{qiblaData.distance_km.toLocaleString(undefined, { maximumFractionDigits: 0 })}</span> km away
        </p>
      </div>

    </div>
  {/if}
</div>

<style>
  .perspective-1000 {
    perspective: 1000px;
  }
  .transform-style-3d {
    transform-style: preserve-3d;
  }
</style>
