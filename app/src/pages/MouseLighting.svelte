<script lang="ts">
  import { onDestroy } from "svelte";
  import { api, debounce, errorText, type LedEffect, type LedZone, type MouseInfo, type ProfileSettings } from "../lib/api";
  import { ledZoneName } from "../lib/keys";

  interface Props {
    info: MouseInfo;
    settings: ProfileSettings;
    onpreview: () => void;
  }

  let { info, settings = $bindable(), onpreview }: Props = $props();
  let error = $state<string | null>(null);

  type Mode = LedEffect["mode"];
  // Codici HID++ degli effetti: 0 spento, 1 fisso, 10 respiro, 3 ciclo colori.
  const MODES: { mode: Exclude<Mode, "raw">; id: number; label: string }[] = [
    { mode: "off", id: 0, label: "Off" },
    { mode: "fixed", id: 1, label: "Fixed" },
    { mode: "breathing", id: 10, label: "Breathing" },
    { mode: "cycle", id: 3, label: "Color cycle" },
  ];

  const preview = debounce((zone: number, effect: LedEffect) => {
    api.previewLed(zone, effect).then(
      () => {
        error = null;
        onpreview();
      },
      (e) => (error = errorText(e)),
    );
  }, 120);

  function set(zone: LedZone, effect: LedEffect) {
    settings.leds[zone.index] = effect;
    preview(zone.index, effect);
  }

  function colorOf(effect: LedEffect): string {
    return effect.mode === "fixed" || effect.mode === "breathing" ? effect.color : "#ff5200";
  }

  function changeMode(zone: LedZone, mode: Mode) {
    const current = settings.leds[zone.index];
    const color = colorOf(current);
    const period = "period_ms" in current ? current.period_ms : 4000;
    const brightness = "brightness" in current ? current.brightness : 100;
    switch (mode) {
      case "off": return set(zone, { mode: "off" });
      case "fixed": return set(zone, { mode: "fixed", color });
      case "breathing": return set(zone, { mode: "breathing", color, period_ms: period, brightness });
      case "cycle": return set(zone, { mode: "cycle", period_ms: period, brightness });
    }
  }

  function patch(zone: LedZone, change: Partial<{ color: string; period_ms: number; brightness: number }>) {
    set(zone, { ...settings.leds[zone.index], ...change } as LedEffect);
  }

  const num = (e: Event) => Number((e.target as HTMLInputElement).value);
  onDestroy(() => preview.cancel());
</script>

<div class="col gap">
  <p class="muted small">
    Changes are previewed on the mouse right away and become permanent with “Save to mouse”.
    The preview uses the connected mouse even when you are editing a profile that is not active.
  </p>
  {#if error}<div class="notice error">{error}</div>{/if}

  {#each info.led_zones as zone (zone.index)}
    {@const effect = settings.leds[zone.index]}
    <div class="card col">
      <h2>{ledZoneName(zone.location)}</h2>
      {#if effect.mode === "raw"}
        <p class="muted small">Effect set by another program: pick one to replace it.</p>
      {/if}
      <div class="segmented">
        {#each MODES.filter((m) => zone.effects.includes(m.id)) as m (m.mode)}
          <button class:active={effect.mode === m.mode} onclick={() => changeMode(zone, m.mode)}>{m.label}</button>
        {/each}
      </div>

      {#if effect.mode === "fixed" || effect.mode === "breathing"}
        <label class="field">
          Color
          <input type="color" value={effect.color} oninput={(e) => patch(zone, { color: (e.target as HTMLInputElement).value })} />
        </label>
      {/if}
      {#if effect.mode === "breathing" || effect.mode === "cycle"}
        <div class="row sliders">
          <label class="field grow">
            Cycle duration: {(effect.period_ms / 1000).toFixed(1)} s
            <input type="range" min="1000" max="20000" step="250" value={effect.period_ms}
              oninput={(e) => patch(zone, { period_ms: num(e) })} />
          </label>
          <label class="field grow">
            Brightness: {effect.brightness}%
            <input type="range" min="1" max="100" step="1" value={effect.brightness}
              oninput={(e) => patch(zone, { brightness: num(e) })} />
          </label>
        </div>
      {/if}
    </div>
  {/each}
</div>

<style>
  .gap {
    gap: 16px;
  }

  .segmented {
    align-self: flex-start;
  }

  input[type="color"] {
    width: 80px;
    height: 36px;
    padding: 2px;
  }

  .sliders {
    gap: 30px;
  }
</style>
