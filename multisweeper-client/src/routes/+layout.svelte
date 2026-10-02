<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import type { LayoutProps } from './$types';
	import { onDestroy } from 'svelte';
	import { browser } from '$app/environment';
	import Background from '$lib/components/Background.svelte';

	let { children, data }: LayoutProps = $props();
	const { ms } = $derived(data);

	onDestroy(() => {
		if (browser) void ms.quit();
	});
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<Background />

<div class="w-lg mx-auto bg-white">
	<div class="flex flex-col items-center justify-center h-screen w-full">
		{@render children()}
	</div>
</div>
