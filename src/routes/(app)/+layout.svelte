<script lang="ts">
  import { onMount } from "svelte";
  import { rcd } from "$lib/stores/rcd.svelte";
  import { transfers } from "$lib/stores/transfers.svelte";
  import { settings } from "$lib/stores/settings.svelte";
  import { text } from "$lib/text/index.svelte";

  let { children } = $props();

  onMount(() => {
    void text.load();
    void rcd.start();
    void transfers.start();
    void settings.load();
  });
</script>

{@render children()}
