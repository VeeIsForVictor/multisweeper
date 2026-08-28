<script lang="ts">
	import type { LobbyState } from "$lib/multisweeper/state";

    interface Props {
        state: LobbyState
        reveal: (x: number, y: number) => void
        flag: (x: number, y: number) => void
        quit: () => void
    };

    const { state, reveal, flag, quit }: Props = $props();
</script>