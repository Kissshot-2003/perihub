<script lang="ts">
  import { onMount } from "svelte";
  import { api, debounce, errorText, type KeyboardLighting, type Rgb } from "../lib/api";
  import { ZONES } from "../lib/effects";
  import ZoneStrip from "../components/ZoneStrip.svelte";

  interface Props {
    connected: boolean;
  }

  let { connected }: Props = $props();

  type Kind = KeyboardLighting["type"];
  const KINDS: { kind: Kind; label: string; hint: string }[] = [
    { kind: "static", label: "Static", hint: "One color per zone" },
    { kind: "gradient", label: "Gradient", hint: "From one color to another" },
    { kind: "breathing", label: "Breathing", hint: "Slow pulse" },
    { kind: "color_cycle", label: "Color cycle", hint: "All zones change together" },
    { kind: "wave", label: "Wave", hint: "Scrolling rainbow" },
    { kind: "hardware_rainbow", label: "Built-in rainbow", hint: "Works even when PeriHub is closed" },
    { kind: "off", label: "Off", hint: "Lights off" },
  ];

  const PRESETS: Rgb[] = ["#ff5200", "#ff0000", "#ffdc00", "#00ff6a", "#00dcff", "#2f5bff", "#8000ff", "#ff00b4", "#ffffff"];

  let loaded = $state(false);
  let kind = $state<Kind>("static");
  let zones = $state<Rgb[]>(Array(ZONES).fill("#ff5200"));
  let from = $state<Rgb>("#ff0000");
  let to = $state<Rgb>("#2f5bff");
  let period = $state(3000);
  let reverse = $state(false);
  let brightness = $state(16);
  let error = $state<string | null>(null);

  const lighting = $derived.by((): KeyboardLighting => {
    switch (kind) {
      case "static": return { type: "static", colors: [...zones] };
      case "gradient": return { type: "gradient", from, to };
      case "breathing": return { type: "breathing", colors: [...zones], period_ms: period };
      case "color_cycle": return { type: "color_cycle", period_ms: period };
      case "wave": return { type: "wave", period_ms: period, reverse };
      case "hardware_rainbow": return { type: "hardware_rainbow" };
      case "off": return { type: "off" };
    }
  });

  const pushLighting = debounce((l: KeyboardLighting) => {
    api.setLighting(l).then(() => (error = null), (e) => (error = errorText(e)));
  }, 60);
  const pushBrightness = debounce((level: number) => {
    api.setBrightness(level).then(() => (error = null), (e) => (error = errorText(e)));
  }, 60);

  $effect(() => {
    const l = lighting;
    if (loaded) pushLighting(l);
  });

  onMount(async () => {
    const config = await api.keyboardConfig();
    const l = config.lighting;
    kind = l.type;
    if (l.type === "static" || l.type === "breathing") zones = [...l.colors];
    if (l.type === "gradient") {
      from = l.from;
      to = l.to;
    }
    if ("period_ms" in l) period = l.period_ms;
    if (l.type === "wave") reverse = l.reverse;
    brightness = config.brightness;
    // La luminosità si può cambiare anche da tastiera (SteelSeries + F11/F12): leggiamo quella reale.
    if (connected) {
      try {
        brightness = await api.readBrightness();
      } catch {
        /* resta quella salvata */
      }
    }
    loaded = true;
  });

  function setAll(color: Rgb) {
    zones = Array(ZONES).fill(color);
  }

  function onBrightness(event: Event) {
    brightness = Number((event.target as HTMLInputElement).value);
    pushBrightness(brightness);
  }

  const usesZones = $derived(kind === "static" || kind === "breathing");
  const usesPeriod = $derived(kind === "breathing" || kind === "color_cycle" || kind === "wave");
</script>

<div class="col gap">
  <ZoneStrip {lighting} {brightness} />

  {#if error}<div class="notice error">{error}</div>{/if}

  <div class="kinds">
    {#each KINDS as k (k.kind)}
      <button class="kind" class:active={kind === k.kind} onclick={() => (kind = k.kind)}>
        <strong>{k.label}</strong>
        <span class="small muted">{k.hint}</span>
      </button>
    {/each}
  </div>

  <div class="card col">
    {#if usesZones}
      <h3>Zone colors (left to right)</h3>
      <div class="zones">
        {#each zones as _, i (i)}
          <label class="zone">
            <input type="color" bind:value={zones[i]} />
            <span class="small muted">Zone {i + 1}</span>
          </label>
        {/each}
      </div>
      <div class="row wrap">
        <span class="small muted">All zones:</span>
        {#each PRESETS as preset (preset)}
          <button class="swatch" style:background={preset} aria-label="Color {preset}" onclick={() => setAll(preset)}></button>
        {/each}
        <input type="color" value={zones[0]} oninput={(e) => setAll((e.target as HTMLInputElement).value)} />
      </div>
    {/if}

    {#if kind === "gradient"}
      <div class="row">
        <label class="field">Start color <input type="color" bind:value={from} /></label>
        <span class="muted">→</span>
        <label class="field">End color <input type="color" bind:value={to} /></label>
      </div>
    {/if}

    {#if usesPeriod}
      <label class="field">
        Cycle duration: {(period / 1000).toFixed(1)} s
        <input type="range" min="500" max="15000" step="100" bind:value={period} />
      </label>
    {/if}

    {#if kind === "wave"}
      <label class="row small"><input type="checkbox" bind:checked={reverse} /> Reverse direction (right to left)</label>
    {/if}

    {#if kind === "hardware_rainbow"}
      <p class="muted small">Generated by the keyboard itself: it keeps running even if you close PeriHub.</p>
    {:else if kind !== "off" && kind !== "static" && kind !== "gradient"}
      <p class="muted small">Animated effects are rendered by PeriHub: keep it running in the system tray.</p>
    {/if}

    <label class="field">
      Brightness: {Math.round((brightness / 16) * 100)}%
      <input type="range" min="0" max="16" step="1" value={brightness} oninput={onBrightness} />
    </label>
  </div>
</div>

<style>
  .gap {
    gap: 18px;
  }

  .kinds {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(170px, 1fr));
    gap: 8px;
  }

  .kind {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    padding: 12px 14px;
    text-align: left;
  }

  .kind.active {
    border-color: var(--accent);
    background: #2a1a12;
  }

  .zones {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 8px;
  }

  .zone {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  input[type="color"] {
    width: 100%;
    min-width: 44px;
    height: 36px;
    padding: 2px;
    cursor: pointer;
  }

  .swatch {
    width: 26px;
    height: 26px;
    padding: 0;
    border-radius: 50%;
    border: 2px solid var(--line);
  }

  .row input[type="color"] {
    width: 44px;
  }

  .field input[type="range"] {
    max-width: 420px;
  }
</style>
