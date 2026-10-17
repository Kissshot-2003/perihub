<script lang="ts">
  import { onMount } from "svelte";
  import { api, type DeviceStatus } from "./lib/api";
  import Keyboard from "./pages/Keyboard.svelte";
  import Mouse from "./pages/Mouse.svelte";
  import Settings from "./pages/Settings.svelte";

  type Page = "keyboard" | "mouse" | "settings";
  const PAGES: Page[] = ["keyboard", "mouse", "settings"];

  // Navigazione nell'indirizzo (#pagina oppure #pagina:scheda), così una pagina si può aprire direttamente.
  function fromHash(): [Page, string | undefined] {
    const [p, tab] = location.hash.slice(1).split(":");
    return [PAGES.includes(p as Page) ? (p as Page) : "keyboard", tab || undefined];
  }

  let [initialPage, initialTab] = fromHash();
  let page = $state<Page>(initialPage);
  let tab = $state<string | undefined>(initialTab);

  function go(next: Page) {
    page = next;
    tab = undefined;
    history.replaceState(null, "", `#${next}`);
  }
  let status = $state<DeviceStatus>({ keyboard: null, mouse: null, keyboard_error: null, mouse_error: null });

  onMount(() => {
    api.status().then((s) => (status = s));
    const onHash = () => ([page, tab] = fromHash());
    window.addEventListener("hashchange", onHash);
    const unlisten = api.onStatus((s) => (status = s));
    return () => {
      unlisten.then((fn) => fn());
      window.removeEventListener("hashchange", onHash);
    };
  });
</script>

<div class="shell">
  <nav>
    <div class="brand">
      <span class="logo"></span>
      <span>PeriHub</span>
    </div>

    <h3>Dispositivi</h3>
    <button class:active={page === "keyboard"} onclick={() => go("keyboard")}>
      <span class="dot" class:on={!!status.keyboard}></span>
      <span class="col tight">
        <span>Apex 3 TKL</span>
        <span class="small muted">{status.keyboard ? "Collegata" : "Non collegata"}</span>
      </span>
    </button>
    <button class:active={page === "mouse"} onclick={() => go("mouse")}>
      <span class="dot" class:on={!!status.mouse}></span>
      <span class="col tight">
        <span>G502 HERO</span>
        <span class="small muted">{status.mouse ? "Collegato" : "Non collegato"}</span>
      </span>
    </button>

    <div class="grow"></div>
    <button class:active={page === "settings"} onclick={() => go("settings")}>Impostazioni</button>
  </nav>

  <main>
    {#if page === "keyboard"}
      {#key tab}<Keyboard info={status.keyboard} error={status.keyboard_error} initialTab={tab} />{/key}
    {:else if page === "mouse"}
      {#key tab}<Mouse info={status.mouse} error={status.mouse_error} initialTab={tab} />{/key}
    {:else}
      <Settings {status} />
    {/if}
  </main>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 220px 1fr;
    height: 100vh;
  }

  nav {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 18px 12px;
    background: var(--panel);
    border-right: 1px solid var(--line);
  }

  nav h3 {
    margin: 18px 8px 6px;
  }

  nav button {
    display: flex;
    align-items: center;
    gap: 12px;
    text-align: left;
    background: transparent;
    border: 1px solid transparent;
    padding: 10px 12px;
  }

  nav button.active {
    background: var(--panel-2);
    border-color: var(--line);
  }

  .tight {
    gap: 2px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    font-weight: 700;
    font-size: 17px;
    padding: 0 8px;
  }

  .logo {
    width: 22px;
    height: 22px;
    border-radius: 7px;
    background: linear-gradient(135deg, #ff5a1f, #d63bff 55%, #2f8bff);
  }

  .dot {
    width: 9px;
    height: 9px;
    border-radius: 50%;
    background: #4a4f5e;
    flex: none;
  }

  .dot.on {
    background: var(--ok);
    box-shadow: 0 0 8px var(--ok);
  }

  main {
    overflow: auto;
    padding: 26px 30px;
  }
</style>
