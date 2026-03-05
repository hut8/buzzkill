<script lang="ts">
	import {
		Compass,
		Gauge,
		Mountain,
		Radio,
		ArrowUp,
		ArrowDown,
		Minus,
		Wifi,
		Bluetooth,
		Clock
	} from '@lucide/svelte';
	import type { Drone } from '$lib/types';

	let drones = $state<Drone[]>([]);
	let error = $state<string | null>(null);
	let loading = $state(true);

	async function fetchDrones() {
		try {
			const res = await fetch('/api/drones');
			if (!res.ok) throw new Error(`HTTP ${res.status}`);
			drones = await res.json();
			error = null;
		} catch (e) {
			error = e instanceof Error ? e.message : 'Failed to fetch';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		fetchDrones();
		const interval = setInterval(fetchDrones, 2000);
		return () => clearInterval(interval);
	});

	function formatDuration(secs: number): string {
		if (secs < 60) return `${Math.round(secs)}s`;
		if (secs < 3600) return `${Math.floor(secs / 60)}m ${Math.round(secs % 60)}s`;
		return `${Math.floor(secs / 3600)}h ${Math.floor((secs % 3600) / 60)}m`;
	}

	function formatSpeed(ms: number): string {
		const kmh = ms * 3.6;
		return `${kmh.toFixed(1)} km/h`;
	}

	function rssiColor(rssi: number): string {
		if (rssi >= -50) return 'text-green-400';
		if (rssi >= -70) return 'text-yellow-400';
		return 'text-red-400';
	}

	function rssiBarWidth(rssi: number): number {
		// Map -100..0 dBm to 0..100%
		return Math.max(0, Math.min(100, ((rssi + 100) / 100) * 100));
	}

	function rssiBarColor(rssi: number): string {
		if (rssi >= -50) return 'bg-green-500';
		if (rssi >= -70) return 'bg-yellow-500';
		return 'bg-red-500';
	}
</script>

<svelte:head>
	<title>Buzzkill - Drone Monitor</title>
</svelte:head>

<div class="container mx-auto max-w-5xl p-4">
	<div class="mb-6 flex items-center justify-between">
		<h1 class="text-2xl font-bold">Observed Drones</h1>
		<div class="flex items-center gap-2 text-sm opacity-70">
			<Radio class="h-4 w-4 animate-pulse text-green-400" />
			<span>Live &middot; {drones.length} active</span>
		</div>
	</div>

	{#if loading}
		<div class="flex justify-center py-20">
			<div class="preset-spinner-dash animate-spin"></div>
		</div>
	{:else if error}
		<div class="preset-filled-error-500 rounded p-4">
			Failed to connect: {error}
		</div>
	{:else if drones.length === 0}
		<div class="rounded-lg border border-surface-500/20 p-12 text-center">
			<Radio class="mx-auto mb-4 h-12 w-12 opacity-30" />
			<p class="text-lg opacity-50">No drones detected</p>
			<p class="mt-1 text-sm opacity-30">Listening for Remote ID broadcasts...</p>
		</div>
	{:else}
		<div class="grid gap-4">
			{#each drones as drone (drone.mac)}
				<div class="card preset-filled-surface-500 rounded-lg p-4">
					<div class="flex items-start justify-between gap-4">
						<!-- Left: Identity -->
						<div class="min-w-0 flex-1">
							<div class="flex items-center gap-2">
								{#if drone.transport === 'ble'}
									<Bluetooth class="h-4 w-4 shrink-0 text-blue-400" />
								{:else}
									<Wifi class="h-4 w-4 shrink-0 text-purple-400" />
								{/if}
								<span class="truncate font-mono text-sm opacity-60">{drone.mac}</span>
							</div>

							{#if drone.basic_id}
								<div class="mt-1">
									<span class="text-lg font-semibold">{drone.basic_id.ua_id || 'Unknown ID'}</span>
									<span class="ml-2 rounded bg-surface-700 px-1.5 py-0.5 text-xs">
										{drone.basic_id.ua_type}
									</span>
								</div>
							{/if}

							{#if drone.operator_id}
								<div class="mt-0.5 text-sm opacity-60">
									Operator: {drone.operator_id.operator_id}
								</div>
							{/if}
						</div>

						<!-- Right: Signal + Timing -->
						<div class="shrink-0 text-right">
							<div class="flex items-center justify-end gap-1.5">
								<div class="h-2 w-16 overflow-hidden rounded-full bg-surface-700">
									<div
										class="h-full rounded-full transition-all {rssiBarColor(drone.rssi)}"
										style="width: {rssiBarWidth(drone.rssi)}%"
									></div>
								</div>
								<span class="font-mono text-sm {rssiColor(drone.rssi)}">{drone.rssi} dBm</span>
							</div>
							<div class="mt-1 flex items-center justify-end gap-1 text-xs opacity-50">
								<Clock class="h-3 w-3" />
								<span>{formatDuration(drone.last_seen_secs_ago)} ago</span>
							</div>
							<div class="text-xs opacity-40">
								tracked {formatDuration(drone.first_seen_secs_ago)} &middot; {drone.msg_count} msgs
							</div>
						</div>
					</div>

					<!-- Location data -->
					{#if drone.location}
						<div
							class="mt-3 grid grid-cols-2 gap-x-6 gap-y-1 border-t border-surface-500/30 pt-3 text-sm sm:grid-cols-4"
						>
							<div class="flex items-center gap-1.5">
								<Compass class="h-3.5 w-3.5 shrink-0 opacity-50" />
								<span class="opacity-60">Bearing</span>
								<span class="ml-auto font-mono">{drone.location.direction.toFixed(0)}&deg;</span>
							</div>
							<div class="flex items-center gap-1.5">
								<Gauge class="h-3.5 w-3.5 shrink-0 opacity-50" />
								<span class="opacity-60">Speed</span>
								<span class="ml-auto font-mono"
									>{formatSpeed(drone.location.speed_horizontal)}</span
								>
							</div>
							<div class="flex items-center gap-1.5">
								<Mountain class="h-3.5 w-3.5 shrink-0 opacity-50" />
								<span class="opacity-60">Alt</span>
								<span class="ml-auto font-mono"
									>{drone.location.altitude_geodetic.toFixed(0)}m</span
								>
							</div>
							<div class="flex items-center gap-1.5">
								{#if drone.location.speed_vertical > 0.5}
									<ArrowUp class="h-3.5 w-3.5 shrink-0 opacity-50" />
								{:else if drone.location.speed_vertical < -0.5}
									<ArrowDown class="h-3.5 w-3.5 shrink-0 opacity-50" />
								{:else}
									<Minus class="h-3.5 w-3.5 shrink-0 opacity-50" />
								{/if}
								<span class="opacity-60">V/S</span>
								<span class="ml-auto font-mono"
									>{drone.location.speed_vertical.toFixed(1)} m/s</span
								>
							</div>
							<div class="col-span-2 flex items-center gap-1.5 sm:col-span-4">
								<span class="opacity-60">Position</span>
								<span class="ml-auto font-mono text-xs">
									{drone.location.latitude.toFixed(6)}, {drone.location.longitude.toFixed(6)}
								</span>
							</div>
							{#if drone.location.height_above_takeoff > -999}
								<div class="col-span-2 flex items-center gap-1.5">
									<span class="opacity-60">Height AGL</span>
									<span class="ml-auto font-mono"
										>{drone.location.height_above_takeoff.toFixed(0)}m</span
									>
								</div>
							{/if}
						</div>
					{/if}

					<!-- System / operator location -->
					{#if drone.system}
						<div
							class="mt-2 border-t border-surface-500/30 pt-2 text-xs opacity-50"
						>
							Operator position: {drone.system.operator_latitude.toFixed(6)}, {drone.system.operator_longitude.toFixed(6)}
							{#if drone.system.area_radius > 0}
								&middot; Area radius: {drone.system.area_radius}m
							{/if}
						</div>
					{/if}
				</div>
			{/each}
		</div>
	{/if}
</div>
