<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText, type KeyBinding, type KeyCatalog, type KeyChange, type Keymap } from "../lib/api";
  import { bindingLabel, keyLabel, LAYOUT_HEIGHT, LAYOUT_WIDTH, TKL_ISO, type KeyCap } from "../lib/keys";
  import ConfirmButton from "../components/ConfirmButton.svelte";

  const UNIT = 46;
  const STEELSERIES_KEY = 0xf0;
  // Funzioni di fabbrica del livello SteelSeries (uguali a perihub-core/src/apex/keys.rs).
  const FACTORY_FUNCTIONS: Record<number, KeyBinding> = {
    0x42: { type: "function", code: 1, param: 0 },
    0x43: { type: "function", code: 2, param: 1 },
    0x44: { type: "function", code: 3, param: 1 },
    0x45: { type: "function", code: 4, param: 1 },
    0xe3: { type: "function", code: 5, param: 0 },
  };

  let catalog = $state<KeyCatalog | null>(null);
  let keymap = $state<Keymap | null>(null);
  let layer = $state<0 | 1>(0);
  let selected = $state<number | null>(null);
  /** Modifiche non ancora salvate, per livello: usage → nuova funzione. */
  let drafts = $state<[Record<number, KeyBinding>, Record<number, KeyBinding>]>([{}, {}]);
  let busy = $state<string | null>("Lettura della mappa dei tasti…");
  let error = $state<string | null>(null);
  let message = $state<string | null>(null);

  onMount(async () => {
    try {
      [catalog, keymap] = await Promise.all([api.keyCatalog(), api.keymap()]);
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = null;
    }
  });

  function factory(l: number, usage: number): KeyBinding {
    if (l === 0) return { type: "key", usage };
    return FACTORY_FUNCTIONS[usage] ?? { type: "none" };
  }

  const same = (a: KeyBinding | undefined, b: KeyBinding | undefined) => JSON.stringify(a) === JSON.stringify(b);

  function saved(l: number, usage: number): KeyBinding | undefined {
    if (!keymap) return undefined;
    return (l === 0 ? keymap.normal : keymap.steelseries)[usage];
  }

  function current(l: number, usage: number): KeyBinding | undefined {
    return drafts[l][usage] ?? saved(l, usage);
  }

  function remappable(cap: KeyCap): boolean {
    return !cap.locked && cap.usage !== STEELSERIES_KEY && !!catalog?.sources.includes(cap.usage);
  }

  function setBinding(usage: number, binding: KeyBinding) {
    const next = { ...drafts[layer] };
    if (same(binding, saved(layer, usage))) delete next[usage];
    else next[usage] = binding;
    drafts[layer] = next;
  }

  function onTarget(event: Event) {
    if (selected === null) return;
    const value = (event.target as HTMLSelectElement).value;
    if (value === "") return;
    setBinding(selected, value === "factory" ? factory(layer, selected) : { type: "key", usage: Number(value) });
  }

  const pending = $derived(
    drafts.flatMap((d, l) =>
      Object.entries(d).map(([usage, binding]): KeyChange => ({ layer: l, source: Number(usage), binding })),
    ),
  );

  async function save() {
    busy = "Salvataggio sulla tastiera…";
    error = message = null;
    try {
      keymap = await api.applyKeymap(pending);
      drafts = [{}, {}];
      message = "Mappa dei tasti salvata nella tastiera.";
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = null;
    }
  }

  async function resetAll() {
    busy = "Ripristino dei tasti di fabbrica…";
    error = message = null;
    try {
      keymap = await api.resetKeymap();
      drafts = [{}, {}];
      message = "Tutti i tasti sono tornati ai valori di fabbrica.";
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = null;
    }
  }

  type Group = { name: string; keys: { usage: number; label: string }[] };
  const groups = $derived.by((): Group[] => {
    if (!catalog) return [];
    const ranges: [string, (u: number) => boolean][] = [
      ["Lettere", (u) => u >= 0x04 && u <= 0x1d],
      ["Numeri e simboli", (u) => (u >= 0x1e && u <= 0x38 && ![0x28, 0x29, 0x2a, 0x2b, 0x2c].includes(u)) || u === 0x64],
      ["Tasti speciali", (u) => [0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x39, 0x65].includes(u)],
      ["Funzione F1–F24", (u) => (u >= 0x3a && u <= 0x45) || (u >= 0x68 && u <= 0x73)],
      ["Navigazione", (u) => u >= 0x46 && u <= 0x52],
      ["Modificatori", (u) => u >= 0xe0 && u <= 0xe7],
      ["Tastierino numerico", (u) => (u >= 0x53 && u <= 0x63) || u === 0x67 || u === 0x85],
      ["Internazionali", (u) => u >= 0x87 && u <= 0x91],
    ];
    return ranges.map(([name, test]) => ({
      name,
      keys: catalog!.targets.filter((k) => test(k.usage)).map((k) => ({ usage: k.usage, label: keyLabel(k.usage) })),
    }));
  });

  function capClass(cap: KeyCap): string {
    if (!remappable(cap)) return "locked";
    const binding = current(layer, cap.usage);
    if (drafts[layer][cap.usage]) return "draft";
    if (!same(binding, factory(layer, cap.usage))) return "custom";
    return "";
  }

  function capSub(cap: KeyCap): string {
    const binding = current(layer, cap.usage);
    if (layer === 0) return binding && !same(binding, factory(0, cap.usage)) ? `→ ${bindingLabel(binding)}` : "";
    return bindingLabel(binding);
  }

  const selectedBinding = $derived(selected === null ? undefined : current(layer, selected));
  const selectedValue = $derived(selectedBinding?.type === "key" ? String(selectedBinding.usage) : "");
</script>

<div class="col gap">
  <div class="row">
    <div class="segmented">
      <button class:active={layer === 0} onclick={() => (layer = 0)}>Normale</button>
      <button class:active={layer === 1} onclick={() => (layer = 1)}>Con tasto SteelSeries</button>
    </div>
    <span class="muted small grow">
      {layer === 0
        ? "Cosa fa ogni tasto quando lo premi."
        : "Cosa fa ogni tasto mentre tieni premuto il tasto SteelSeries."}
    </span>
  </div>

  {#if error}<div class="notice error">{error}</div>{/if}
  {#if message}<div class="notice ok">{message}</div>{/if}
  {#if busy}<div class="notice info">{busy}</div>{/if}

  {#if keymap}
    <div class="board" style:width="{LAYOUT_WIDTH * UNIT}px" style:height="{LAYOUT_HEIGHT * UNIT}px">
      {#each TKL_ISO as cap (cap.usage)}
        <button
          class="cap {capClass(cap)}"
          class:selected={selected === cap.usage}
          disabled={!remappable(cap) || !!busy}
          style:left="{cap.x * UNIT}px"
          style:top="{cap.y * UNIT}px"
          style:width="{cap.w * UNIT - 4}px"
          style:height="{cap.h * UNIT - 4}px"
          title={cap.locked ? "Questo tasto non è rimappabile dal firmware" : keyLabel(cap.usage)}
          onclick={() => (selected = cap.usage)}
        >
          <span>{keyLabel(cap.usage)}</span>
          {#if capSub(cap)}<span class="sub">{capSub(cap)}</span>{/if}
        </button>
      {/each}
    </div>

    <div class="row legend small muted">
      <span><i class="custom"></i> Personalizzato</span>
      <span><i class="draft"></i> Da salvare</span>
      <span><i class="locked"></i> Non rimappabile</span>
    </div>

    <div class="card row editor">
      {#if selected === null}
        <span class="muted">Seleziona un tasto per cambiarne la funzione.</span>
      {:else}
        <div class="col tight">
          <span class="small muted">Tasto</span>
          <strong>{keyLabel(selected)}{layer === 1 ? " (con SteelSeries)" : ""}</strong>
        </div>
        <div class="col tight">
          <span class="small muted">Funzione</span>
          <strong>{selectedBinding?.type === "none" ? "Come senza SteelSeries" : bindingLabel(selectedBinding)}</strong>
        </div>
        <label class="field grow">
          Nuova funzione
          <select value={selectedValue} onchange={onTarget}>
            <option value="" disabled>Scegli un tasto…</option>
            <option value="factory">Valore di fabbrica</option>
            {#each groups as group (group.name)}
              <optgroup label={group.name}>
                {#each group.keys as key (key.usage)}
                  <option value={String(key.usage)}>{key.label}</option>
                {/each}
              </optgroup>
            {/each}
          </select>
        </label>
      {/if}
    </div>

    <div class="row">
      <ConfirmButton class="danger" disabled={!!busy} onconfirm={resetAll} confirmLabel="Ripristino tutti i tasti?">
        Ripristina tutti di fabbrica
      </ConfirmButton>
      <div class="grow"></div>
      {#if pending.length}
        <span class="muted small">{pending.length} modifiche da salvare</span>
        <button disabled={!!busy} onclick={() => (drafts = [{}, {}])}>Annulla</button>
      {/if}
      <button class="primary" disabled={!pending.length || !!busy} onclick={save}>Salva sulla tastiera</button>
    </div>
    <p class="muted small">
      La mappa viene salvata nella memoria della tastiera e funziona anche senza PeriHub, su qualsiasi computer.
      Prima di ogni salvataggio viene creata una copia di sicurezza.
    </p>
  {/if}
</div>

<style>
  .gap {
    gap: 16px;
  }

  .tight {
    gap: 2px;
  }

  .board {
    position: relative;
    padding: 12px;
    box-sizing: content-box;
    border-radius: var(--radius);
    background: #0b0c10;
    border: 1px solid var(--line);
  }

  .cap {
    position: absolute;
    margin: 12px 0 0 12px;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    padding: 2px;
    font-size: 11px;
    line-height: 1.1;
    border-radius: 6px;
    background: #1d2029;
    border: 1px solid #2c3140;
    overflow: hidden;
  }

  .cap .sub {
    font-size: 9px;
    color: var(--accent-2);
  }

  .cap.custom {
    border-color: var(--accent);
  }

  .cap.draft {
    border-color: var(--warn);
    background: #2a2412;
  }

  .cap.locked {
    opacity: 0.35;
  }

  .cap.selected {
    outline: 2px solid var(--accent);
    background: #2a1a12;
  }

  .legend {
    gap: 18px;
  }

  .legend i {
    display: inline-block;
    width: 10px;
    height: 10px;
    border-radius: 3px;
    margin-right: 4px;
    border: 1px solid;
  }

  .legend i.custom { border-color: var(--accent); }
  .legend i.draft { border-color: var(--warn); background: #2a2412; }
  .legend i.locked { opacity: 0.35; background: #1d2029; }

  .editor {
    gap: 28px;
    min-height: 72px;
  }

  .editor select {
    max-width: 320px;
  }
</style>
