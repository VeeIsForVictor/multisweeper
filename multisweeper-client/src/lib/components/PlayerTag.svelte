<script lang="ts">
	import type { HTMLAttributes } from 'svelte/elements';
	import BinocularsIcon from 'virtual:iconify/mdi/binoculars?mode=svg';
	import StarIcon from 'virtual:iconify/mdi/star?mode=svg';
	import ControllerClassicIcon from 'virtual:iconify/mdi/controller-classic?mode=svg';
	import { cn } from '$lib/utils';

	type PlayerTagType = 'spectator' | 'host' | 'playing' | 'none';

	interface Props extends HTMLAttributes<HTMLSpanElement> {
		playerId: string;
		isYou: boolean;
		type?: PlayerTagType;
	}

	let { playerId, isYou, type = 'none', class: className, ...rest }: Props = $props();
	const ICON_HEIGHT = '1.5em';
</script>

<span {...rest} class={cn('flex flex-row items-center gap-1 rounded border px-2 py-1 text-sm', className)}>
	{isYou ? 'You' : playerId}
	{#if type === 'host'}
		<StarIcon height={ICON_HEIGHT} aria-label="Host" />
	{:else if type === 'playing'}
		<ControllerClassicIcon height={ICON_HEIGHT} aria-label="Playing" />
	{:else if type === 'spectator'}
		<BinocularsIcon height={ICON_HEIGHT} aria-label="Spectator" />
	{/if}
</span>
