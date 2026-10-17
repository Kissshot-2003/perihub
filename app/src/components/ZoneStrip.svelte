<script lang="ts">
  import { onMount } from "svelte";
  import type { KeyboardLighting } from "../lib/api";
  import { previewFrame } from "../lib/effects";

  interface Props {
    lighting: KeyboardLighting;
    brightness: number;
  }

  let { lighting, brightness }: Props = $props();
  let colors = $state<string[]>([]);

  onMount(() => {
    const start = performance.now();
    let frame = 0;
    const draw = (now: number) => {
      colors = previewFrame(lighting, now - start);
      frame = requestAnimationFrame(draw);
    };
    frame = requestAnimationFrame(draw);
    return () => cancelAnimationFrame(frame);
  });
</script>

<div class="strip" style:opacity={0.25 + 0.75 * (brightness / 16)}>
  {#each colors as color, i (i)}
    <div class="zone" style:background={color} style:box-shadow="0 0 18px {color}"></div>
  {/each}
</div>

<style>
  .strip {
    display: grid;
    grid-template-columns: repeat(8, 1fr);
    gap: 6px;
    padding: 14px;
    border-radius: var(--radius);
    background: #08090c;
    border: 1px solid var(--line);
  }

  .zone {
    height: 34px;
    border-radius: 6px;
  }
</style>
