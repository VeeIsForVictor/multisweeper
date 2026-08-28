<script lang="ts">
	import { resolve } from "$app/paths";
	import type { State } from "$lib/multisweeper/state";
    import type { PageProps } from "./$types";

    const { data, params }: PageProps = $props();
    const { ms } = $derived(data);

    let joinPromise: Promise<State> | null = $state(null);

    $effect(() => {
        if (ms.state.type !== 'connecting') {
            joinPromise = ms.joinRoom(params.roomId);
        }
    })

</script>

{#if ms.state.type !== 'connecting'}
    {#await joinPromise}
        <h1>Connecting...</h1>
    {:then state} 
        {@const room = state?.type === 'lobby' ? state.roomId : ''}
        <h1>Lobby: {room}</h1>
    {:catch error}
        <h1 class="text-red-600">{error.message}</h1>
        <a href={resolve('/')}>Return to Room List?</a>
    {/await}
{/if}