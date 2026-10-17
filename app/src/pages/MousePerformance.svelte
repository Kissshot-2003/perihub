<script lang="ts">
  import { onDestroy } from "svelte";
  import { api, debounce, type MouseInfo, type ProfileSettings } from "../lib/api";
  import { reportRateLabel } from "../lib/keys";

  interface Props {
    info: MouseInfo;
    settings: ProfileSettings;
    onpreview: () => void;
  }

  let { info, settings = $bindable(), onpreview }: Props = $props();

  const caps = $derived(info.dpi);
  const MAX_LEVELS = 5;

  const snap = (value: number) => {
    const step = caps.step || 1;
    const snapped = Math.round((value - caps.min) / step) * step + caps.min;
    return Math.min(caps.max, Math.max(caps.min, snapped));
  };

  const preview = debounce((dpi: number) => {
    api.previewDpi(dpi).then(onpreview, () => {});
  }, 150);

  function setLevel(i: number, value: number) {
    settings.dpi_levels[i] = snap(value);
    preview(settings.dpi_levels[i]);
  }

  function addLevel() {
    const last = settings.dpi_levels[settings.dpi_levels.length - 1] ?? 800;
    settings.dpi_levels.push(snap(Math.min(caps.max, last * 2)));
  }

  function removeLevel(i: number) {
    settings.dpi_levels.splice(i, 1);
    const n = settings.dpi_levels.length;
    if (settings.default_dpi_index >= n) settings.default_dpi_index = n - 1;
    if (settings.shift_dpi_index >= n) settings.shift_dpi_index = 0;
  }

  // Scala logaritmica per il cursore: più precisione ai DPI bassi, quelli usati davvero.
  const toSlider = (dpi: number) => Math.log(dpi / caps.min) / Math.log(caps.max / caps.min);
  const fromSlider = (t: number) => caps.min * Math.pow(caps.max / caps.min, t);
  onDestroy(() => preview.cancel());
</script>

<div class="col gap">
  <div class="card col">
    <div class="row">
      <h2 class="grow">Livelli DPI</h2>
      <span class="muted small">{caps.min}–{caps.max} DPI, passo {caps.step}</span>
    </div>
    <p class="muted small">
      Il livello <b>predefinito</b> è quello attivo quando selezioni il profilo; il livello <b>sniper</b> vale
      mentre tieni premuto il pulsante DPI Shift. Muovendo un cursore il mouse prova subito quei DPI.
    </p>

    <div class="levels">
      <span class="small muted">Livello</span>
      <span class="small muted">DPI</span>
      <span class="small muted"></span>
      <span class="small muted center">Predefinito</span>
      <span class="small muted center">Sniper</span>
      <span></span>
      {#each settings.dpi_levels as dpi, i (i)}
        <strong>{i + 1}</strong>
        <input
          type="number"
          min={caps.min}
          max={caps.max}
          step={caps.step || 1}
          value={dpi}
          onchange={(e) => setLevel(i, Number((e.target as HTMLInputElement).value))}
        />
        <input
          type="range"
          min="0"
          max="1"
          step="0.001"
          value={toSlider(dpi)}
          oninput={(e) => setLevel(i, fromSlider(Number((e.target as HTMLInputElement).value)))}
        />
        <input class="center" type="radio" name="default-dpi" checked={settings.default_dpi_index === i}
          onchange={() => (settings.default_dpi_index = i)} />
        <input class="center" type="radio" name="shift-dpi" checked={settings.shift_dpi_index === i}
          disabled={!info.onboard.has_dpi_shift} onchange={() => (settings.shift_dpi_index = i)} />
        <button class="ghost" disabled={settings.dpi_levels.length <= 1} onclick={() => removeLevel(i)} aria-label="Rimuovi livello">✕</button>
      {/each}
    </div>
    <div>
      <button disabled={settings.dpi_levels.length >= MAX_LEVELS} onclick={addLevel}>+ Aggiungi livello</button>
    </div>
  </div>

  <div class="card col">
    <h2>Frequenza di polling</h2>
    <p class="muted small">Quante volte al secondo il mouse comunica la sua posizione. 1000 Hz è il valore consigliato per giocare.</p>
    <div class="segmented">
      {#each [...info.report_rates_ms].sort((a, b) => a - b) as ms (ms)}
        <button class:active={settings.report_rate_ms === ms} onclick={() => (settings.report_rate_ms = ms)}>
          {reportRateLabel(ms)}
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .gap {
    gap: 16px;
  }

  .levels {
    display: grid;
    grid-template-columns: 50px 110px 1fr 90px 70px 40px;
    align-items: center;
    gap: 10px 14px;
  }

  .levels input[type="number"] {
    width: 100%;
  }

  .center {
    justify-self: center;
    text-align: center;
  }

  .segmented {
    align-self: flex-start;
  }
</style>
