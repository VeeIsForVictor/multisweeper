<script lang="ts">
    import { type PageProps } from "./$types";
    export const { data }: PageProps = $props();
    
    const { ms } = $derived(data);
    const state = $derived(ms.state);

    $effect(() => {
        if (state.type === 'ready') {
            ms.queryRooms();

            setInterval(ms.queryRooms, 1000);
        }
    })
</script>

<h1>Welcome to Multisweeper!</h1>

<div class="flex flex-col m-8 items-center gap-2">
    {#if state.type === 'connecting'}
        <h1>Connecting to Multisweeper Server</h1>
    {:else if state.type === 'ready'}
        <h1>Querying server for lobbies...</h1>
    {:else if state.type === 'landed'}
            <button>Host Game</button>
            or
            <h3>Join an Existing Room</h3>
            <div class="grid grid-rows-3">
                {#each state.lobbies as lobby (lobby)}
                    <div class="flex flex-col">
                        {lobby}
                    </div>
                {:else}
                    <h4>No lobbies available. Why not make your own?</h4>
                {/each}
            </div>
    {:else}
        <h1 class="text-red-500">This shouldn't be appearing, oops!</h1>
    {/if}
</div>