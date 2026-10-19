<script lang="ts">
  // Editor for a macro stored in the mouse: kind, steps recorded from the keyboard or added by hand.
  import { untrack } from "svelte";
  import type { KeyInfo, Macro, MacroKind, MacroStep } from "../lib/api";
  import { CONSUMER_KEYS, keyLabel, MOUSE_CLICKS } from "../lib/keys";
  import {
    DEFAULT_DELAY_MS, MACRO_KINDS, MAX_DELAY_MS, MODIFIER_KEYS, recordedStep, releaseOf, stepLabel,
  } from "../lib/macros";

  interface Props {
    content: Macro;
    keys: KeyInfo[];
    /** Name of the button the macro is for. */
    button: string;
    onsave: (content: Macro) => void;
    oncancel: () => void;
  }

  let { content, keys, button, onsave, oncancel }: Props = $props();

  const initial: Macro = untrack(() => JSON.parse(JSON.stringify(content)));
  let kind = $state<MacroKind>(initial.kind);
  let press = $state<MacroStep[]>(initial.press);
  let release = $state<MacroStep[]>(initial.release);
  /** List that recording and "Add" write to, for "Press and release". */
  let part = $state<"press" | "release">("press");
  let defaultDelay = $state(DEFAULT_DELAY_MS);
  let recordDelays = $state(true);
  let recording = $state(false);
  /** Presses recorded and not released yet. */
  let held: MacroStep[] = [];
  let lastEvent = 0;

  type AddKind = "key" | "mouse" | "media" | "scroll" | "delay";
  let addKind = $state<AddKind>("key");
  /** "k:<usage>" for a key, "m:<bit>" for a modifier. */
  let addKey = $state("k:4");
  let addButton = $state(1);
  let addMedia = $state(CONSUMER_KEYS[0].usage);
  let addScroll = $state(1);

  const hint = $derived(MACRO_KINDS.find((k) => k.kind === kind)?.hint ?? "");
  const target = () => (kind === "sequence" && part === "release" ? release : press);
  const delay = (ms = defaultDelay): MacroStep => ({ type: "delay", ms });

  function setKind(next: MacroKind) {
    // The release part exists only in "Press and release": keep its steps at the end.
    if (kind === "sequence" && next !== "sequence" && release.length) {
      if (press.length) press.push(delay());
      press.push(...release);
      release = [];
    }
    kind = next;
    part = "press";
  }

  /** Adds a step, after the default pause if the previous step is not a pause. */
  function append(steps: MacroStep[], step: MacroStep) {
    if (steps.length && steps[steps.length - 1].type !== "delay" && step.type !== "delay") steps.push(delay());
    steps.push(step);
  }

  /** A press, the default pause and the release. */
  function addTap(down: MacroStep) {
    const steps = target();
    append(steps, down);
    steps.push(delay(), releaseOf(down)!);
  }

  function add() {
    switch (addKind) {
      case "key": {
        const [type, value] = addKey.split(":");
        const n = Number(value);
        return addTap({ type: "key_down", modifiers: type === "m" ? n : 0, key: type === "k" ? n : 0 });
      }
      case "mouse": return addTap({ type: "mouse_down", buttons: 1 << (addButton - 1) });
      case "media": return addTap({ type: "consumer_down", usage: addMedia });
      case "scroll": return append(target(), { type: "wheel", amount: addScroll });
      case "delay": return target().push(delay());
    }
  }

  function move(steps: MacroStep[], i: number, by: number) {
    const [step] = steps.splice(i, 1);
    steps.splice(i + by, 0, step);
  }

  function setDelay(steps: MacroStep[], i: number, e: Event) {
    const ms = Math.round(Number((e.target as HTMLInputElement).value));
    if (Number.isFinite(ms)) steps[i] = delay(Math.min(MAX_DELAY_MS, Math.max(0, ms)));
  }

  function startRecording() {
    held = [];
    lastEvent = 0;
    recording = true;
  }

  function stopRecording() {
    if (!recording) return;
    recording = false;
    // Keys still down when recording stops are released, so the macro never leaves one pressed.
    for (const step of held) target().push(delay(), releaseOf(step)!);
    held = [];
  }

  const sameKey = (a: MacroStep, b: MacroStep) =>
    "key" in a && "key" in b && a.key === b.key && a.modifiers === b.modifiers;

  function onkey(e: KeyboardEvent, down: boolean) {
    if (!recording) {
      if (down && e.key === "Escape") oncancel();
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    if (down && e.repeat) return;
    const step = recordedStep(e.code, down);
    if (!step) return;
    if (down) {
      held.push(step);
    } else {
      // A key that was already down when recording started.
      const i = held.findIndex((h) => sameKey(h, step));
      if (i < 0) return;
      held.splice(i, 1);
    }
    const now = performance.now();
    const steps = target();
    if (steps.length && steps[steps.length - 1].type !== "delay") {
      const ms = recordDelays && lastEvent ? Math.round(now - lastEvent) : defaultDelay;
      steps.push(delay(Math.min(MAX_DELAY_MS, Math.max(1, ms))));
    }
    lastEvent = now;
    steps.push(step);
  }

  const problem = $derived.by(() => {
    if (kind === "forever") return "PeriHub cannot save a macro that repeats until the profile changes: choose another kind.";
    const empty = kind === "sequence" ? !press.length && !release.length : !press.length;
    return empty ? "Add at least one step." : null;
  });

  /** Keys and buttons still down when the macro ends. */
  const leftDown = $derived.by(() => {
    const down: MacroStep[] = [];
    for (const step of kind === "sequence" ? [...press, ...release] : press) {
      if (releaseOf(step)) down.push(step);
      const i = down.findIndex((d) => JSON.stringify(releaseOf(d)) === JSON.stringify(step));
      if (i >= 0) down.splice(i, 1);
    }
    return down.map((s) => stepLabel(s).replace(/^Press /, ""));
  });

  function save() {
    stopRecording();
    onsave({
      kind,
      press: $state.snapshot(press),
      release: kind === "sequence" ? $state.snapshot(release) : [],
    });
  }

  const value = (e: Event) => (e.target as HTMLSelectElement).value;
</script>

<svelte:window onkeydown={(e) => onkey(e, true)} onkeyup={(e) => onkey(e, false)} onblur={stopRecording} />

{#snippet stepList(steps: MacroStep[], name: "press" | "release")}
  <ol class="steps" class:target={kind === "sequence" && part === name}>
    {#each steps as step, i (i)}
      <li>
        <span class="index muted small">{i + 1}</span>
        {#if step.type === "delay"}
          <label class="row delay">
            Wait
            <input type="number" min="0" max={MAX_DELAY_MS} value={step.ms} onchange={(e) => setDelay(steps, i, e)} />
            ms
          </label>
        {:else}
          <span>{stepLabel(step)}</span>
        {/if}
        <div class="grow"></div>
        <button class="ghost tiny" aria-label="Move up" title="Move up" disabled={i === 0 || recording}
          onclick={() => move(steps, i, -1)}>↑</button>
        <button class="ghost tiny" aria-label="Move down" title="Move down" disabled={i === steps.length - 1 || recording}
          onclick={() => move(steps, i, 1)}>↓</button>
        <button class="ghost tiny" aria-label="Remove" title="Remove" disabled={recording}
          onclick={() => steps.splice(i, 1)}>×</button>
      </li>
    {:else}
      <li class="muted small">No steps yet: record keys or add steps below.</li>
    {/each}
  </ol>
{/snippet}

<div class="backdrop">
  <div class="dialog col" role="dialog" aria-modal="true" aria-labelledby="macro-title">
    <h2 id="macro-title">Macro for “{button}”</h2>

    <div class="col tight">
      <div class="segmented">
        {#each MACRO_KINDS as k (k.kind)}
          <button class:active={kind === k.kind} disabled={recording} onclick={() => setKind(k.kind)}>{k.label}</button>
        {/each}
      </div>
      <span class="muted small">{hint}</span>
    </div>
    {#if kind === "forever"}
      <div class="notice info">
        This macro repeats until the mouse changes profile: pressing the button again starts it over. PeriHub cannot
        save this kind, because nothing else stops it. Choose another kind to keep its steps.
      </div>
    {/if}

    <div class="row wrap">
      {#if recording}
        <button class="primary" onclick={stopRecording}>Stop recording</button>
        <span class="rec small">● Type the keys of the macro, then click Stop.</span>
      {:else}
        <button onclick={startRecording}>Record keys</button>
        <label class="row small"><input type="checkbox" bind:checked={recordDelays} /> Record the pauses</label>
      {/if}
      <div class="grow"></div>
      <label class="row small">
        Standard pause
        <input class="ms" type="number" min="1" max={MAX_DELAY_MS} bind:value={defaultDelay} disabled={recording} />
        ms
      </label>
    </div>

    <div class="lists">
      {#if kind === "sequence"}
        <div class="col tight">
          <button class="part" class:active={part === "press"} onclick={() => (part = "press")} disabled={recording}>
            When pressed
          </button>
          {@render stepList(press, "press")}
        </div>
        <div class="col tight">
          <button class="part" class:active={part === "release"} onclick={() => (part = "release")} disabled={recording}>
            When released
          </button>
          {@render stepList(release, "release")}
        </div>
      {:else}
        {@render stepList(press, "press")}
      {/if}
    </div>

    <div class="row wrap add">
      <select value={addKind} onchange={(e) => (addKind = value(e) as AddKind)} disabled={recording}>
        <option value="key">Key</option>
        <option value="mouse">Mouse click</option>
        <option value="media">Media key</option>
        <option value="scroll">Scroll</option>
        <option value="delay">Pause</option>
      </select>
      {#if addKind === "key"}
        <select bind:value={addKey} disabled={recording}>
          <optgroup label="Modifiers">
            {#each MODIFIER_KEYS as m (m.bit)}<option value={`m:${m.bit}`}>{m.label}</option>{/each}
          </optgroup>
          <optgroup label="Keys">
            {#each keys as key (key.usage)}<option value={`k:${key.usage}`}>{keyLabel(key.usage)}</option>{/each}
          </optgroup>
        </select>
      {:else if addKind === "mouse"}
        <select value={String(addButton)} onchange={(e) => (addButton = Number(value(e)))} disabled={recording}>
          {#each MOUSE_CLICKS as c (c.button)}<option value={String(c.button)}>{c.label}</option>{/each}
        </select>
      {:else if addKind === "media"}
        <select value={String(addMedia)} onchange={(e) => (addMedia = Number(value(e)))} disabled={recording}>
          {#each CONSUMER_KEYS as c (c.usage)}<option value={String(c.usage)}>{c.label}</option>{/each}
        </select>
      {:else if addKind === "scroll"}
        <select value={String(addScroll)} onchange={(e) => (addScroll = Number(value(e)))} disabled={recording}>
          <option value="1">Up</option>
          <option value="-1">Down</option>
        </select>
      {/if}
      <button onclick={add} disabled={recording}>Add</button>
      <span class="muted small">
        {#if addKind === "delay"}A pause of the standard length.{:else if addKind === "scroll"}One notch.{:else}Press and
          release, with the standard pause between them.{/if}
      </span>
    </div>
    <span class="muted small">
      Shortcuts the system keeps for itself, such as Alt+Tab or the Windows key, may not reach PeriHub while recording:
      add them with “Key”.
    </span>

    {#if leftDown.length}
      <div class="notice info">Still pressed when the macro ends: {leftDown.join(", ")}. Add the release if it is not meant to stay down.</div>
    {/if}

    <footer class="row">
      <button class="ghost" disabled={recording} onclick={() => { press = []; release = []; }}>Clear</button>
      <div class="grow"></div>
      {#if problem}<span class="warn small">{problem}</span>{/if}
      <button onclick={oncancel}>Cancel</button>
      <button class="primary" disabled={!!problem} onclick={save}>Done</button>
    </footer>
    <span class="muted small">The macro is stored in the mouse with “Save to mouse” and works without PeriHub.</span>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    padding: 24px;
    background: rgb(0 0 0 / 0.55);
  }

  .dialog {
    width: min(760px, 100%);
    max-height: 100%;
    overflow-y: auto;
    gap: 14px;
    padding: 22px;
    border-radius: var(--radius);
    background: var(--panel);
    border: 1px solid var(--line);
  }

  .tight {
    gap: 6px;
  }

  .segmented {
    align-self: flex-start;
  }

  .lists {
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 1fr;
    gap: 12px;
  }

  .steps {
    list-style: none;
    margin: 0;
    padding: 6px;
    min-height: 120px;
    max-height: 300px;
    overflow-y: auto;
    border: 1px solid var(--line);
    border-radius: 8px;
    background: var(--panel-2);
  }

  .steps.target {
    border-color: var(--accent);
  }

  .steps li {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 3px 4px;
    min-height: 30px;
  }

  .index {
    width: 22px;
    text-align: right;
  }

  .delay input {
    width: 80px;
    padding: 3px 6px;
  }

  .ms {
    width: 80px;
  }

  .part {
    align-self: flex-start;
    border: none;
    background: transparent;
    padding: 2px 0;
    color: var(--muted);
    font-weight: 600;
  }

  .part.active {
    color: var(--accent);
  }

  .tiny {
    padding: 2px 7px;
  }

  .rec {
    color: var(--bad);
  }

  .warn {
    color: var(--warn);
  }

  footer {
    border-top: 1px solid var(--line);
    padding-top: 14px;
  }
</style>
