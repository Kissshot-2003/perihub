<script lang="ts">
  import { onMount } from "svelte";
  import { api, type ButtonAction, type KeyInfo, type MouseInfo, type ProfileSettings } from "../lib/api";
  import { MOUSE_BUTTON_NAMES } from "../lib/keys";
  import ButtonActionEditor from "../components/ButtonActionEditor.svelte";

  interface Props {
    info: MouseInfo;
    settings: ProfileSettings;
  }

  let { info, settings = $bindable() }: Props = $props();
  let layer = $state<"normal" | "gshift">("normal");
  let keys = $state<KeyInfo[]>([]);

  onMount(async () => {
    keys = (await api.keyCatalog()).targets;
  });

  const list = $derived(layer === "normal" ? settings.buttons : settings.gshift_buttons);
  const hasGShiftButton = $derived(settings.buttons.some((b) => b.type === "special" && b.function === "g_shift"));

  function update(i: number, action: ButtonAction) {
    if (layer === "normal") settings.buttons[i] = action;
    else settings.gshift_buttons[i] = action;
  }
</script>

<div class="col gap">
  {#if info.onboard.has_gshift}
    <div class="row">
      <div class="segmented">
        <button class:active={layer === "normal"} onclick={() => (layer = "normal")}>Normale</button>
        <button class:active={layer === "gshift"} onclick={() => (layer = "gshift")}>G-Shift</button>
      </div>
      <span class="muted small">
        {#if layer === "gshift"}
          Funzioni usate mentre tieni premuto il pulsante a cui hai assegnato «G-Shift».
          {#if !hasGShiftButton}<b class="warn">Nessun pulsante ha ancora la funzione G-Shift.</b>{/if}
        {:else}
          Funzioni normali dei pulsanti.
        {/if}
      </span>
    </div>
  {/if}

  <div class="card buttons">
    {#each list as action, i (i)}
      <div class="name">
        <strong>{MOUSE_BUTTON_NAMES[i] ?? `Pulsante ${i + 1}`}</strong>
        <span class="small muted">Pulsante {i + 1}</span>
      </div>
      <ButtonActionEditor {action} {keys} allowDisabledHint={layer === "gshift"} onchange={(a) => update(i, a)} />
    {/each}
  </div>
</div>

<style>
  .gap {
    gap: 16px;
  }

  .buttons {
    display: grid;
    grid-template-columns: 200px 1fr;
    align-items: center;
    gap: 12px 20px;
  }

  .name {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .warn {
    color: var(--warn);
  }
</style>
