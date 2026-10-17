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

<h1>Settings</h1>

<div class="col gap">
  {#if error}<div class="notice error">{error}</div>{/if}

  <div class="card col">
    <h2>Startup</h2>
    {#if settings}
      <label class="row">
        <input type="checkbox" checked={settings.autostart}
          onchange={(e) => update({ autostart: (e.target as HTMLInputElement).checked })} />
        Start PeriHub when I log in (hidden in the system tray)
      </label>
      <label class="row">
        <input type="checkbox" checked={settings.start_hidden}
          onchange={(e) => update({ start_hidden: (e.target as HTMLInputElement).checked })} />
        Always start hidden
      </label>
    {/if}
    <p class="muted small">
      Closing the window does not quit PeriHub: it keeps running to animate the keyboard lighting and to
      reapply colors when the keyboard is plugged back in. Reopen it from the tray icon or by launching it again.
    </p>
  </div>

  <div class="card col">
    <h2>Where settings are stored</h2>
    <ul class="small">
      <li><b>Mouse</b>: DPI, polling, buttons and lighting are stored in the G502's memory and work without PeriHub.</li>
      <li><b>Keyboard</b>: the key mapping is stored in the Apex's memory. Lighting is not kept by the keyboard, so PeriHub reapplies it.</li>
      <li>Before every write to a device, a backup is saved in the <code>perihub/backups</code> folder of your user configuration.</li>
    </ul>
  </div>

  <div class="card col">
    <h2>Devices</h2>
    <div class="small">
      <div>Apex 3 TKL: {status.keyboard ? `firmware ${status.keyboard.firmware}` : status.keyboard_error ?? "not connected"}</div>
      <div>G502 HERO: {status.mouse ? status.mouse.firmware.map((f) => `${f.name} ${f.version}`).join(", ") : status.mouse_error ?? "not connected"}</div>
    </div>
    <p class="muted small">
      Quit SteelSeries GG and Logitech G HUB while using PeriHub: two programs talking to the same device get in each other's way.
    </p>
  </div>

  <div class="card col">
    <h2>Credits</h2>
    <p class="muted small">
      Protocols reconstructed thanks to the work of OpenRGB (Apex lighting), libratbag (Logitech HID++) and
      gorkemgun/apex-3-tkl (Apex key mapping). PeriHub is not affiliated with SteelSeries or Logitech.
    </p>
  </div>

  <div>
    <button class="danger" onclick={() => api.quit()}>Quit PeriHub</button>
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
