<script lang="ts">
  import { untrack } from "svelte";
  import type { KeyboardInfo } from "../lib/api";
  import KeyboardLighting from "./KeyboardLighting.svelte";
  import KeyboardKeys from "./KeyboardKeys.svelte";

  interface Props {
    info: KeyboardInfo | null;
    error: string | null;
    initialTab?: string;
  }

  let { info, error, initialTab }: Props = $props();
  let tab = $state<"lighting" | "keys">(untrack(() => initialTab) === "keys" ? "keys" : "lighting");
</script>

<header class="row">
  <div class="col tight grow">
    <h1>SteelSeries Apex 3 TKL</h1>
    <span class="muted small">
      {#if info}Firmware {info.firmware}{:else}Non collegata: le impostazioni di illuminazione verranno applicate al collegamento{/if}
    </span>
  </div>
</header>

{#if error && !info}
  <div class="notice error">{error}</div>
{/if}

<div class="tabs">
  <button class:active={tab === "lighting"} onclick={() => (tab = "lighting")}>Illuminazione</button>
  <button class:active={tab === "keys"} onclick={() => (tab = "keys")}>Tasti</button>
</div>

<section>
  {#if tab === "lighting"}
    <KeyboardLighting connected={!!info} />
  {:else if info}
    <KeyboardKeys />
  {:else}
    <div class="notice info">Collega la tastiera per modificare la mappa dei tasti.</div>
  {/if}
</section>

<style>
  header {
    margin-bottom: 18px;
  }

  .tight {
    gap: 4px;
  }

  section {
    padding-top: 20px;
  }

  .notice {
    margin-bottom: 14px;
  }
</style>
