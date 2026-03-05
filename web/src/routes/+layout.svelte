<script lang="ts">
	import '../app.css';
	import { AppBar } from '@skeletonlabs/skeleton-svelte';
	import { Radar, Bluetooth, Wifi, Sun, Moon } from '@lucide/svelte';
	import type { ScanStatus } from '$lib/types';

	let { children } = $props();
	let status = $state<ScanStatus | null>(null);
	let dark = $state(true);

	$effect(() => {
		const stored = localStorage.getItem('theme');
		if (stored === 'light') {
			dark = false;
			document.documentElement.classList.remove('dark');
		}
	});

	$effect(() => {
		fetch('/api/status')
			.then((r) => r.json())
			.then((s) => (status = s))
			.catch(() => {});
	});

	function toggleTheme() {
		dark = !dark;
		if (dark) {
			document.documentElement.classList.add('dark');
			localStorage.setItem('theme', 'dark');
		} else {
			document.documentElement.classList.remove('dark');
			localStorage.setItem('theme', 'light');
		}
	}
</script>

<div class="flex h-full flex-col">
	<AppBar>
		<AppBar.Toolbar>
			<AppBar.Lead>
				<a href="/" class="flex items-center gap-2 text-xl font-bold">
					<Radar class="h-6 w-6" />
					Buzzkill
				</a>
			</AppBar.Lead>
			<AppBar.Trail class="flex items-center gap-4">
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
				<button
					class="btn btn-sm preset-tonal-surface-500"
					onclick={toggleTheme}
					title={dark ? 'Switch to light mode' : 'Switch to dark mode'}
				>
					{#if dark}
						<Sun size={18} />
					{:else}
						<Moon size={18} />
					{/if}
				</button>
			</AppBar.Trail>
		</AppBar.Toolbar>
	</AppBar>

	<main class="flex-1 overflow-auto">
		{@render children()}
	</main>
</div>
