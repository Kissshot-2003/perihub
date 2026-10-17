<script lang="ts">
  import { onMount } from "svelte";
  import { api, errorText, type AppSettings, type DeviceStatus } from "../lib/api";

  interface Props {
    status: DeviceStatus;
  }

  let { status }: Props = $props();
  let settings = $state<AppSettings | null>(null);
  let error = $state<string | null>(null);

  onMount(async () => {
    settings = await api.appSettings();
  });

  async function update(change: Partial<AppSettings>) {
    if (!settings) return;
    try {
      settings = await api.setAppSettings({ ...settings, ...change });
      error = null;
    } catch (e) {
      error = errorText(e);
    }
  }
</script>

<h1>Impostazioni</h1>

<div class="col gap">
  {#if error}<div class="notice error">{error}</div>{/if}

  <div class="card col">
    <h2>Avvio</h2>
    {#if settings}
      <label class="row">
        <input type="checkbox" checked={settings.autostart}
          onchange={(e) => update({ autostart: (e.target as HTMLInputElement).checked })} />
        Avvia PeriHub all'accesso (nascosto nell'area di notifica)
      </label>
      <label class="row">
        <input type="checkbox" checked={settings.start_hidden}
          onchange={(e) => update({ start_hidden: (e.target as HTMLInputElement).checked })} />
        Avvia sempre nascosto
      </label>
    {/if}
    <p class="muted small">
      Chiudere la finestra non chiude PeriHub: resta attivo per gli effetti di luce della tastiera e per
      riapplicare i colori quando la ricolleghi. Riaprilo dall'icona nell'area di notifica o avviandolo di nuovo.
    </p>
  </div>

  <div class="card col">
    <h2>Dove vengono salvate le impostazioni</h2>
    <ul class="small">
      <li><b>Mouse</b>: DPI, polling, pulsanti e luci vanno nella memoria del G502 e funzionano anche senza PeriHub.</li>
      <li><b>Tastiera</b>: la mappa dei tasti va nella memoria della Apex. Le luci invece sono temporanee e PeriHub le riapplica.</li>
      <li>Prima di ogni scrittura viene salvata una copia di sicurezza nella cartella <code>perihub/backups</code> della configurazione utente.</li>
    </ul>
  </div>

  <div class="card col">
    <h2>Dispositivi</h2>
    <div class="small">
      <div>Apex 3 TKL: {status.keyboard ? `firmware ${status.keyboard.firmware}` : status.keyboard_error ?? "non collegata"}</div>
      <div>G502 HERO: {status.mouse ? status.mouse.firmware.map((f) => `${f.name} ${f.version}`).join(", ") : status.mouse_error ?? "non collegato"}</div>
    </div>
    <p class="muted small">
      Chiudi SteelSeries GG e Logitech G HUB mentre usi PeriHub: due programmi che parlano con lo stesso dispositivo si ostacolano.
    </p>
  </div>

  <div class="card col">
    <h2>Riconoscimenti</h2>
    <p class="muted small">
      Protocolli ricostruiti grazie al lavoro di OpenRGB (illuminazione Apex), libratbag (Logitech HID++) e
      gorkemgun/apex-3-tkl (mappa tasti Apex). PeriHub non è affiliato a SteelSeries né a Logitech.
    </p>
  </div>

  <div>
    <button class="danger" onclick={() => api.quit()}>Esci da PeriHub</button>
  </div>
</div>

<style>
  h1 {
    margin-bottom: 18px;
  }

  .gap {
    gap: 16px;
  }

  ul {
    margin: 0;
    padding-left: 18px;
    line-height: 1.7;
  }
</style>
