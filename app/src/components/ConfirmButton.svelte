<script lang="ts">
  // Two-click button for destructive actions: the second click confirms, no dialog box.
  import type { Snippet } from "svelte";

  interface Props {
    onconfirm: () => void;
    confirmLabel?: string;
    disabled?: boolean;
    class?: string;
    children: Snippet;
  }

  let { onconfirm, confirmLabel = "Are you sure?", disabled = false, class: klass = "", children }: Props = $props();
  let armed = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  function click() {
    if (armed) {
      clearTimeout(timer);
      armed = false;
      onconfirm();
      return;
    }
    armed = true;
    timer = setTimeout(() => (armed = false), 3000);
  }
</script>

<button class={klass} class:armed {disabled} onclick={click}>
  {#if armed}{confirmLabel}{:else}{@render children()}{/if}
</button>

<style>
  .armed {
    border-color: var(--bad) !important;
    color: var(--bad) !important;
  }
</style>
