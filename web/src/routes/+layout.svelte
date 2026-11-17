<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import AppShell from '$lib/components/shell/AppShell.svelte';
  import { ui } from '$lib/stores/ui.svelte';

  let { children } = $props();

  onMount(() => ui.applyDocumentTheme());
</script>

<AppShell>
  {@render children()}
</AppShell>
