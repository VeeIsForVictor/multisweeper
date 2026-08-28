<script lang="ts">
	import { resolve } from "$app/paths";
	import type { State } from "$lib/multisweeper/state";
	import { onDestroy } from "svelte";
    import type { PageProps } from "./$types";

    const { data, params }: PageProps = $props();
    const { ms } = $derived(data);

    let joinPromise: Promise<State> | null = $state(null);
    let requestedRoomId: string | null = $state(null);

    $effect(() => {
        const roomId = params.roomId;
        if (requestedRoomId === roomId) return;

        requestedRoomId = roomId;
        joinPromise = ms.joinRoom(roomId);
        void joinPromise.catch(() => undefined);
    })

    onDestroy(() => {
        void ms.leaveRoom().catch(() => undefined);
    })

</script>

{#if ms.state.type !== 'connecting'}
    {#await joinPromise}
        <h1>Connecting...</h1>
    {:then state} 
        {@const room = state?.type === 'lobby' ? state.roomId : ms.roomId}
        <h1>Lobby: {room}</h1>
    {:catch error}
        <h1 class="text-red-600">{error.message}</h1>
        <a href={resolve('/')}>Return to Room List?</a>
    {/await}
{/if}
