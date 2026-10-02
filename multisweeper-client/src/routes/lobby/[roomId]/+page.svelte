<script lang="ts">
	import { resolve } from '$app/paths';
	import { goto } from '$app/navigation';
	import { onDestroy } from 'svelte';
	import type { PageProps } from './$types';
	import Game from '$lib/components/Game.svelte';

	const { data, params }: PageProps = $props();
	const { ms } = $derived(data);

	let requestedRoomId: string | undefined;
	let joinError: Error | null = $state(null);

	let gameState = $derived(ms.state);

	$effect(() => {
		const roomId = params.roomId;
		if (requestedRoomId === roomId) return;

		requestedRoomId = roomId;
		joinError = null;
		void ms.joinRoom(roomId).catch((error) => {
			if (requestedRoomId !== roomId) return;
			joinError = error instanceof Error ? error : new Error(String(error));
		});
	});

	onDestroy(() => {
		void ms.leaveRoom().catch(() => undefined);
	});
</script>

{#if ms.state.type === 'connecting'}
	<h1>Connecting...</h1>
{:else if ms.state.type === 'fatal'}
	<h1 class="text-red-600">{ms.state.message}</h1>
	<a href={resolve('/')}>Return to Room List?</a>
{:else if joinError}
	<h1 class="text-red-600">{joinError.message}</h1>
	<a href={resolve('/')}>Return to Room List?</a>
{:else if gameState.type === 'lobby' && gameState.roomId === params.roomId}
	<Game
		lobby={gameState}
		start={(difficulty) => ms.startGame(difficulty)}
		reveal={(x, y) => ms.reveal(x, y)}
		flag={(x, y) => ms.flag(x, y)}
		quit={async () => {
			await ms.leaveRoom();
			await goto(resolve('/'));
		}}
	/>
{:else}
	<h1>Joining lobby...</h1>
{/if}
