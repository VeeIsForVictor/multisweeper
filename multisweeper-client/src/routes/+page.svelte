<script lang="ts">
	import { resolve } from "$app/paths";
	import { onMount } from "svelte";
    import { type PageProps } from "./$types";
	import { goto } from "$app/navigation";
    export const { data }: PageProps = $props();
    
    const { ms } = $derived(data);
    const state = $derived(ms.state);

    onMount(() => {
        const querySchedule = setInterval(() => {
            if (ms.state.type === 'ready' || ms.state.type === 'landed') {
                void ms.queryRooms().catch((error) => {
                    console.error('failed to query rooms', error);
                });
            }
        }, 1000);

        return () => clearInterval(querySchedule);
    });

    async function handleCreateRoom() {
        try {
            const roomId = await ms.createRoom();
            await goto(resolve('/lobby/[roomId]', { roomId }));
        } catch (error) {
            console.error('failed to create room', error);
        }
    }
    
</script>

<h1>Welcome to Multisweeper!</h1>

<div class="flex flex-col m-8 items-center gap-2">
    {#if state.type === 'connecting'}
        <h1>Connecting to Multisweeper Server</h1>
    {:else if state.type === 'ready'}
        <h1>Querying server for lobbies...</h1>
    {:else if state.type === 'landed'}
            <button onclick={handleCreateRoom}>Host Game</button>
            or
            <h3>Join an Existing Room</h3>
            <div class="grid grid-rows-3">
                {#each state.lobbies as lobby (lobby)}
                    <div class="flex flex-col">
                        <a 
                            href={resolve("/lobby/[roomId]", { roomId: lobby })} 
                            title="join room {lobby}"
                        >
                            {lobby}
                        </a>
                    </div>
                {:else}
                    <h4>No lobbies available. Why not make your own?</h4>
                {/each}
            </div>
    {:else}
        <h1 class="text-red-500">This shouldn't be appearing, oops!</h1>
    {/if}
</div>
