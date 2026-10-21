<script lang="ts">
  // Linux: permission to read what is typed on the Apex, needed by reactive lighting and by keys that
  // start software macros. It opens the keyboard to every program of the user, so it comes with a
  // full warning and can be removed. Shows nothing on Windows.
  import { onMount } from "svelte";
  import { api, errorText, type KeyAccess } from "../lib/api";

  interface Props {
    /** Why the permission is needed, shown when it is missing. */
    title: string;
    /** Called after the permission was granted and applied. */
    ongranted?: () => void;
  }

  let { title, ongranted }: Props = $props();
  let keyAccess = $state<KeyAccess | null>(null);
  let understood = $state(false);
  let busy = $state(false);
  let error = $state<string | null>(null);
  /** The rule was just removed: the access granted to the keyboard ends when it is plugged in again. */
  let revoked = $state(false);

  async function check() {
    try {
      keyAccess = await api.keyAccess();
    } catch (e) {
      error = errorText(e);
    }
  }

  async function change(grant: boolean) {
    busy = true;
    error = null;
    try {
      await (grant ? api.grantKeyAccess() : api.revokeKeyAccess());
      understood = false;
      revoked = !grant;
      // udev applies the new permissions a moment later.
      await new Promise((r) => setTimeout(r, 1500));
      await check();
      if (grant) ongranted?.();
    } catch (e) {
      error = errorText(e);
    } finally {
      busy = false;
    }
  }

  onMount(() => {
    if (!navigator.userAgent.includes("Windows")) check();
  });
</script>

{#if keyAccess?.supported && keyAccess.readable === false}
  <div class="notice warn col">
    <strong>{title}</strong>
    <span>Before allowing it, know what it means:</span>
    <ul>
      <li>
        Linux cannot give this permission to PeriHub alone. <b>Every program that runs under your user account</b>
        will be able to read <b>every key you press on the Apex 3 TKL</b>, in any app, even while PeriHub is closed.
      </li>
      <li>
        That includes <b>passwords, private messages and anything else you type</b>. A malicious or compromised
        program (a downloaded script, a game mod, a browser extension with access to your system) could use it
        as a keylogger without you noticing.
      </li>
      <li>
        It turns off, for this keyboard, the protection Wayland normally gives you: apps can only see the keys
        typed into their own windows.
      </li>
      <li>
        PeriHub itself reads key presses only while reactive lighting is on, or while a key of this keyboard starts
        a software macro. It keeps only which zone to light or which macro to start: it never stores, logs or sends
        what you type. The permission, though, stays until you remove it here.
      </li>
    </ul>
    <span>Allow it only if you trust all the software you run. Your password will be asked to install the rule.</span>
    <label class="row">
      <input type="checkbox" bind:checked={understood} />
      I understand that every program running as my user will be able to read what I type on this keyboard.
    </label>
    <div class="row">
      <button class="danger" disabled={!understood || busy} onclick={() => change(true)}>
        {busy ? "Waiting for the password…" : "Allow PeriHub to read key presses"}
      </button>
    </div>
  </div>
{:else if keyAccess?.supported && keyAccess.readable && keyAccess.rule_installed}
  <div class="row small muted">
    <span class="grow">
      Programs running as your user can read key presses on this keyboard, because you allowed it in PeriHub.
    </span>
    <button class="danger" disabled={busy} onclick={() => change(false)}>Remove permission</button>
  </div>
{:else if keyAccess?.supported && keyAccess.readable && revoked}
  <div class="notice info">
    Permission removed. To finish, unplug the keyboard and plug it in again (or restart the computer).
  </div>
{:else if keyAccess?.supported && keyAccess.readable}
  <p class="muted small">
    Another rule on this system (for example OpenRGB's) already lets programs running as your user read key
    presses on this keyboard.
  </p>
{/if}
{#if error}
  <div class="notice error col">
    <span>{error}</span>
    {#if keyAccess}
      <span class="small">You can also run this in a terminal:</span>
      <code>{keyAccess.rule_installed ? keyAccess.manual_remove : keyAccess.manual_install}</code>
    {/if}
  </div>
{/if}

<style>
  .notice.warn {
    gap: 8px;
  }

  .notice ul {
    margin: 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  code {
    user-select: all;
    word-break: break-all;
  }
</style>
