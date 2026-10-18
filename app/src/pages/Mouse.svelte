<script lang="ts">
  import { onDestroy, untrack } from "svelte";
  import { api, errorText, type MouseInfo, type MouseState, type ProfileSettings } from "../lib/api";
  import { reportRateLabel } from "../lib/keys";
  import ConfirmButton from "../components/ConfirmButton.svelte";
  import MousePerformance from "./MousePerformance.svelte";
  import MouseButtons from "./MouseButtons.svelte";
  import MouseLighting from "./MouseLighting.svelte";

  interface Props {
    info: MouseInfo | null;
    error: string | null;
    initialTab?: string;
  }

  let { info, error: connectError, initialTab }: Props = $props();

  let mouse = $state<MouseState | null>(null);
  let selected = $state(1);
  let draft = $state<ProfileSettings | null>(null);
  type Tab = "performance" | "buttons" | "lighting";
  const startTab = untrack(() => initialTab);
  let tab = $state<Tab>(startTab === "buttons" || startTab === "lighting" ? startTab : "performance");
  let busy = $state<string | null>(null);
  let error = $state<string | null>(null);
  let message = $state<string | null>(null);
  /** A preview (DPI or LED) is active on the mouse and must be reverted. */
  let previewing = $state(false);

  const clone = <T,>(value: T): T => JSON.parse(JSON.stringify(value));
  const savedSettings = $derived(mouse?.profiles.find((p) => p.number === selected)?.settings ?? null);
  const dirty = $derived(!!draft && JSON.stringify(draft) !== JSON.stringify(savedSettings));
  const leftClickMissing = $derived(!!draft && !draft.buttons.some((b) => b.type === "mouse" && b.button === 1));

  async function run<T>(label: string, task: () => Promise<T>): Promise<T | undefined> {
    busy = label;
    error = message = null;
    try {
      return await task();
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = null;
    }
  }

  function adopt(next: MouseState, keepSelection = true) {
    mouse = next;
    if (!keepSelection) selected = next.active_profile;
    draft = clone(next.profiles.find((p) => p.number === selected)!.settings);
  }

  let loadedFor: MouseInfo | null = null;
  $effect(() => {
    if (info && info !== loadedFor) {
      loadedFor = info;
      run("Reading profiles from the mouse…", api.mouseState).then((s) => s && adopt(s, false));
    }
    if (!info) {
      loadedFor = null;
      mouse = null;
      draft = null;
    }
  });

  async function revertPreview() {
    if (!previewing) return;
    previewing = false;
    await api.revertPreview().catch(() => {});
  }

  async function selectProfile(number: number) {
    if (number === selected) return;
    await revertPreview();
    selected = number;
    draft = clone(mouse!.profiles.find((p) => p.number === number)!.settings);
    message = null;
  }

  async function save() {
    if (!draft) return;
    const settings = clone(draft);
    const next = await run(`Saving profile ${selected} to the mouse…`, () => api.saveProfile(selected, settings));
    if (next) {
      previewing = false;
      adopt(next);
      message = `Profile ${selected} saved to the mouse.`;
    }
  }

  async function discard() {
    await revertPreview();
    draft = clone(savedSettings!);
  }

  async function factory() {
    const next = await run(`Restoring profile ${selected}…`, () => api.restoreFactory(selected));
    if (next) {
      previewing = false;
      adopt(next);
      message = `Profile ${selected} restored to factory settings.`;
    }
  }

  async function activate(number: number) {
    const next = await run("Switching profile…", () => api.setActiveProfile(number));
    if (next) {
      previewing = false;
      mouse = next;
    }
  }

  async function toggleEnabled(number: number, enabled: boolean) {
    const next = await run("Updating profiles…", () => api.setProfileEnabled(number, enabled));
    if (next) mouse = next;
  }

  async function changeTab(next: typeof tab) {
    if (next === tab) return;
    tab = next;
    await revertPreview();
  }

  // When leaving the page (for another section of the app), the mouse goes back to the saved settings.
  onDestroy(() => {
    if (previewing) api.revertPreview().catch(() => {});
  });
</script>

<header class="col tight">
  <h1>{info ? `Logitech ${info.model}` : "Logitech mouse"}</h1>
  <span class="muted small">
    {#if info}
      Firmware {info.firmware.map((f) => `${f.name} ${f.version}`).join(" · ")}
      {#if mouse}· current DPI {mouse.current_dpi}{/if}
    {:else}
      Not connected
    {/if}
  </span>
</header>

{#if connectError && !info}<div class="notice error">{connectError}</div>{/if}
{#if info && !info.verified}
  <div class="notice info">
    <b>Experimental support.</b> PeriHub has not been tested on the {info.model} yet, so it only reads the mouse:
    you can look at its profiles, but nothing can be changed. A mouse test report on PeriHub's GitHub page helps get
    it fully supported.
  </div>
  {#if info.host_mode}
    <div class="notice info">
      Another program, such as G HUB, left this mouse under software control: it is not using the profiles shown here.
    </div>
  {/if}
{:else if info && !info.writable}
  <div class="notice error">This model uses a profile format that has not been verified yet: editing is disabled.</div>
{/if}

{#if info && mouse && draft}
  <div class="profiles">
    {#each mouse.profiles as profile (profile.number)}
      <div class="profile" class:selected={profile.number === selected} class:disabled={!profile.enabled}>
        <button class="pick" onclick={() => selectProfile(profile.number)} disabled={!!busy}>
          <strong>Profile {profile.number}</strong>
          <span class="small muted">
            {profile.settings.dpi_levels.join(" / ")} DPI · {reportRateLabel(profile.settings.report_rate_ms)}
          </span>
        </button>
        <div class="row small">
          {#if profile.number === mouse.active_profile}
            <span class="chip active">Active</span>
          {:else}
            <button class="ghost tiny" disabled={!!busy || !profile.enabled || !info.verified} onclick={() => activate(profile.number)}>
              Make active
            </button>
          {/if}
          <label class="row tiny-label" title="Disabled profiles are skipped by the profile switch button">
            <input
              type="checkbox"
              checked={profile.enabled}
              disabled={!!busy || !info.writable}
              onchange={(e) => toggleEnabled(profile.number, (e.target as HTMLInputElement).checked)}
            />
            Enabled
          </label>
        </div>
      </div>
    {/each}
  </div>

  {#if error}<div class="notice error">{error}</div>{/if}
  {#if message}<div class="notice ok">{message}</div>{/if}
  {#if busy}<div class="notice info">{busy}</div>{/if}

  <div class="tabs">
    <button class:active={tab === "performance"} onclick={() => changeTab("performance")}>DPI and polling</button>
    <button class:active={tab === "buttons"} onclick={() => changeTab("buttons")}>Buttons</button>
    <button class:active={tab === "lighting"} onclick={() => changeTab("lighting")}>Lighting</button>
  </div>

  <!-- Unverified models are read-only: every control in the editor is disabled. -->
  <fieldset disabled={!info.verified}>
    {#if tab === "performance"}
      <MousePerformance {info} bind:settings={draft} onpreview={() => (previewing = true)} />
    {:else if tab === "buttons"}
      <MouseButtons {info} bind:settings={draft} />
    {:else}
      <MouseLighting {info} bind:settings={draft} onpreview={() => (previewing = true)} />
    {/if}
  </fieldset>

  <footer class="row">
    <ConfirmButton class="danger" disabled={!!busy || !info.writable} onconfirm={factory} confirmLabel="Reset this profile?">
      Factory settings
    </ConfirmButton>
    <div class="grow"></div>
    {#if leftClickMissing}
      <span class="warn small">No button does a left click any more: assign one before saving.</span>
    {:else if dirty}
      <span class="muted small">Unsaved changes</span>
    {/if}
    <button disabled={!dirty || !!busy} onclick={discard}>Discard</button>
    <button class="primary" disabled={!dirty || !!busy || leftClickMissing || !info.writable} onclick={save}>
      Save to mouse
    </button>
  </footer>
{:else if info}
  {#if error}<div class="notice error">{error}</div>{:else}<div class="notice info">{busy ?? "Loading…"}</div>{/if}
{/if}

<style>
  header {
    margin-bottom: 18px;
    gap: 4px;
  }

  .tight {
    gap: 4px;
  }

  .profiles {
    display: grid;
    grid-template-columns: repeat(5, 1fr);
    gap: 8px;
    margin-bottom: 14px;
  }

  .profile {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px;
    border-radius: var(--radius);
    background: var(--panel);
    border: 1px solid var(--line);
  }

  .profile.selected {
    border-color: var(--accent);
  }

  .profile.disabled .pick {
    opacity: 0.5;
  }

  .pick {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 3px;
    background: transparent;
    border: none;
    padding: 2px;
    text-align: left;
  }

  .chip.active {
    color: var(--ok);
    border-color: #1f4f3c;
  }

  .tiny {
    padding: 2px 6px;
    font-size: 12px;
  }

  .tiny-label {
    gap: 4px;
    margin-left: auto;
    color: var(--muted);
  }

  .notice {
    margin-bottom: 12px;
  }

  fieldset {
    border: none;
    margin: 0;
    min-width: 0;
    padding: 20px 0 90px;
  }

  footer {
    position: sticky;
    bottom: -26px;
    margin: 0 -30px -26px;
    padding: 14px 30px;
    background: var(--panel);
    border-top: 1px solid var(--line);
  }

  .warn {
    color: var(--warn);
  }
</style>
