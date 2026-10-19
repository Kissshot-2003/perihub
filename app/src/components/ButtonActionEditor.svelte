<script lang="ts">
  import type { ButtonAction, KeyInfo } from "../lib/api";
  import { CONSUMER_KEYS, keyLabel, macroSummary, MODIFIERS, MOUSE_CLICKS, SPECIAL_FUNCTIONS } from "../lib/keys";

  interface Props {
    action: ButtonAction;
    keys: KeyInfo[];
    allowDisabledHint?: boolean;
    onchange: (action: ButtonAction) => void;
  }

  let { action, keys, allowDisabledHint = false, onchange }: Props = $props();

  type Kind = ButtonAction["type"];
  const KINDS: { kind: Kind; label: string }[] = [
    { kind: "mouse", label: "Mouse click" },
    { kind: "key", label: "Keyboard key" },
    { kind: "consumer", label: "Media key" },
    { kind: "special", label: "Mouse function" },
    { kind: "disabled", label: "Disabled" },
  ];

  function changeKind(kind: Kind) {
    switch (kind) {
      case "mouse": return onchange({ type: "mouse", button: 1 });
      case "key": return onchange({ type: "key", modifiers: 0, key: 0x04 });
      case "consumer": return onchange({ type: "consumer", usage: CONSUMER_KEYS[0].usage });
      case "special": return onchange({ type: "special", function: "dpi_cycle" });
      case "disabled": return onchange({ type: "disabled" });
    }
  }

  function toggleModifier(bit: number, on: boolean) {
    if (action.type !== "key") return;
    onchange({ ...action, modifiers: on ? action.modifiers | bit : action.modifiers & ~bit });
  }

  const value = (e: Event) => (e.target as HTMLSelectElement).value;
</script>

<div class="row wrap editor">
  <select value={action.type} onchange={(e) => changeKind(value(e) as Kind)}>
    {#if action.type === "macro"}<option value="macro" disabled>Macro</option>{/if}
    {#if action.type === "raw"}<option value="raw" disabled>Custom value</option>{/if}
    {#each KINDS as k (k.kind)}
      <option value={k.kind}>{k.label}</option>
    {/each}
  </select>

  {#if action.type === "mouse"}
    <select value={String(action.button)} onchange={(e) => onchange({ type: "mouse", button: Number(value(e)) })}>
      {#each MOUSE_CLICKS as click (click.button)}
        <option value={String(click.button)}>{click.label}</option>
      {/each}
    </select>
  {:else if action.type === "key"}
    {#each MODIFIERS as mod (mod.bit)}
      <label class="row small mod">
        <input type="checkbox" checked={(action.modifiers & mod.bit) !== 0}
          onchange={(e) => toggleModifier(mod.bit, (e.target as HTMLInputElement).checked)} />
        {mod.label}
      </label>
    {/each}
    <span class="muted">+</span>
    <select value={String(action.key)} onchange={(e) => action.type === "key" && onchange({ ...action, key: Number(value(e)) })}>
      {#each keys as key (key.usage)}
        <option value={String(key.usage)}>{keyLabel(key.usage)}</option>
      {/each}
    </select>
  {:else if action.type === "consumer"}
    <select value={String(action.usage)} onchange={(e) => onchange({ type: "consumer", usage: Number(value(e)) })}>
      {#if !CONSUMER_KEYS.some((c) => action.type === "consumer" && c.usage === action.usage)}
        <option value={String(action.usage)}>Code 0x{action.usage.toString(16)}</option>
      {/if}
      {#each CONSUMER_KEYS as c (c.usage)}
        <option value={String(c.usage)}>{c.label}</option>
      {/each}
    </select>
  {:else if action.type === "special"}
    <select value={action.function} onchange={(e) => onchange({ type: "special", function: value(e) as never })}>
      {#each SPECIAL_FUNCTIONS as f (f.value)}
        <option value={f.value}>{f.label}</option>
      {/each}
    </select>
  {:else if action.type === "macro"}
    <span class="small">{macroSummary(action.steps)}</span>
    <span class="muted small">Stored in the mouse: pick a type to replace it.</span>
  {:else if action.type === "raw"}
    <span class="muted small">Set by another program: pick a type to replace it.</span>
  {:else if action.type === "disabled" && allowDisabledHint}
    <span class="muted small">The button does nothing while G-Shift is held.</span>
  {/if}
</div>

<style>
  .editor {
    gap: 8px;
  }

  .mod {
    gap: 4px;
  }
</style>
