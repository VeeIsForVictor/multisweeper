<script lang="ts">
	import type { PageProps } from "./$types";

    const { data, params }: PageProps = $props();
    const { ms, logger } = $derived(data);

    $effect(() => {
        if (ms.state.type !== 'connecting')
            ms.joinRoom(params.roomId).catch(() => {
                logger.warn("join room rejected");
            });
    })
</script>
