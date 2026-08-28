<script lang="ts">
	import { resolve } from "$app/paths";
	import { onDestroy } from "svelte";
	import type { PageProps } from "./$types";

	const { data, params }: PageProps = $props();
	const { ms } = $derived(data);

	let requestedRoomId: string | undefined;
	let joinError: Error | null = $state(null);

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
{:else if ms.state.type === 'lobby' && ms.state.roomId === params.roomId}
    <h1>Lobby: {ms.state.roomId}</h1>
{:else}
    <h1>Joining lobby...</h1>
{/if}
