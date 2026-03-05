<script lang="ts">
	import '../app.css';
	import { AppBar } from '@skeletonlabs/skeleton-svelte';
	import { Radar } from '@lucide/svelte';

	let { children } = $props();
</script>

<div class="flex h-full flex-col">
	<AppBar>
		{#snippet lead()}
			<a href="/" class="flex items-center gap-2 text-xl font-bold">
				<Radar class="h-6 w-6" />
				Buzzkill
			</a>
		{/snippet}
	</AppBar>

	<main class="flex-1 overflow-auto">
		{@render children()}
	</main>
</div>
