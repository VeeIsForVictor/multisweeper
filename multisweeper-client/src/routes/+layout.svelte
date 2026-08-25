<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import type { LayoutProps } from './$types';
	import { onDestroy } from 'svelte';

	let { children, data }: LayoutProps = $props();
	const { ms } = $derived(data);

	onDestroy(() => ms.quit());
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>
<div class="flex flex-col items-center justify-center h-screen w-full">
	{@render children()}
</div>
