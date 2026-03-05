<script lang="ts">
	import '../app.css';
	import { AppBar } from '@skeletonlabs/skeleton-svelte';
	import { Radar, Bluetooth, Wifi } from '@lucide/svelte';
	import type { ScanStatus } from '$lib/types';

	let { children } = $props();
	let status = $state<ScanStatus | null>(null);

	$effect(() => {
		fetch('/api/status')
			.then((r) => r.json())
			.then((s) => (status = s))
			.catch(() => {});
	});
</script>

<div class="flex h-full flex-col">
	<AppBar>
		{#snippet lead()}
			<a href="/" class="flex items-center gap-2 text-xl font-bold">
				<Radar class="h-6 w-6" />
				Buzzkill
			</a>
		{/snippet}
		{#snippet trail()}
			{#if status}
				<div class="flex items-center gap-3 text-sm">
					<span class="flex items-center gap-1" class:opacity-30={!status.bluetooth}>
						<Bluetooth class="h-4 w-4" />
						<span class="hidden sm:inline">BLE</span>
					</span>
					<span class="flex items-center gap-1" class:opacity-30={!status.wifi}>
						<Wifi class="h-4 w-4" />
						<span class="hidden sm:inline">WiFi</span>
					</span>
				</div>
			{/if}
		{/snippet}
	</AppBar>

	<main class="flex-1 overflow-auto">
		{@render children()}
	</main>
</div>
