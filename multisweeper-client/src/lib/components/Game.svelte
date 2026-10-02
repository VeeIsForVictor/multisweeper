<script lang="ts">
	import type { LobbyState } from '$lib/multisweeper/state';
	import type { CellView, ClientDifficulty } from '$lib/protocol';

	interface Props {
		lobby: LobbyState;
		start: (difficulty: ClientDifficulty) => Promise<void>;
		reveal: (x: number, y: number) => Promise<void>;
		flag: (x: number, y: number) => Promise<void>;
		quit: () => Promise<void>;
	}

	const { lobby, start, reveal, flag, quit }: Props = $props();

	let difficulty = $state<ClientDifficulty>('Easy');
	let pendingAction = $state<string | null>(null);
	let actionError = $state<string | null>(null);

	const matchState = $derived(lobby.game.state);
	const game = $derived(lobby.game.game);
	const currentPlayerId = $derived(
		typeof matchState === 'object' ? matchState.Playing.current_player : null
	);
	const localPlayerState = $derived(
		lobby.players.find((player) => player.id === lobby.playerId)?.state ?? null
	);
	const canAct = $derived(
		game?.status === 'Playing' && currentPlayerId === lobby.playerId && pendingAction === null
	);
	const boardColumns = $derived(game?.board[0]?.length ?? 0);

	function describeCell(cell: CellView): string {
		if (cell === 'HiddenCell') return 'Hidden cell';
		if (cell === 'FlaggedCell') return 'Flagged cell';
		if (cell === 'MinedCell') return 'Mine';

		return cell.VisibleCell === 0 ? 'Empty revealed cell' : `${cell.VisibleCell} adjacent mines`;
	}

	function cellContent(cell: CellView): string {
		if (cell === 'HiddenCell') return '';
		if (cell === 'FlaggedCell') return '⚑';
		if (cell === 'MinedCell') return '✹';

		return cell.VisibleCell === 0 ? '' : String(cell.VisibleCell);
	}

	async function perform(action: string, operation: () => Promise<void>) {
		if (pendingAction !== null) return;

		pendingAction = action;
		actionError = null;
		try {
			await operation();
		} catch (error) {
			actionError = error instanceof Error ? error.message : String(error);
		} finally {
			pendingAction = null;
		}
	}

	function revealCell(cell: CellView, x: number, y: number) {
		if (cell === 'FlaggedCell') return;
		if (!canAct) return;
		void perform('reveal', () => reveal(x, y));
	}

	function flagCell(x: number, y: number) {
		if (!canAct) return;
		void perform('flag', () => flag(x, y));
	}

	function startGame() {
		void perform('start', () => start(difficulty));
	}

	function leaveLobby() {
		void perform('leave', quit);
	}
</script>

<section class="flex w-full max-w-4xl flex-col gap-6 p-6" aria-label="Multisweeper game">
	<header class="flex flex-wrap items-center justify-between gap-3">
		<div>
			<h2 class="text-2xl">Room {lobby.roomId}</h2>
			<p>{lobby.players.length} player{lobby.players.length === 1 ? '' : 's'}</p>
		</div>
		<button onclick={leaveLobby} disabled={pendingAction !== null}>Leave room</button>
	</header>

	<div class="flex flex-wrap gap-2" aria-label="Players">
		{#each lobby.players as player (player.id)}
			<span class="rounded border px-2 py-1">
				{player.id === lobby.playerId ? 'You' : player.id}: {player.state}
			</span>
		{/each}
	</div>

	{#if matchState === 'Waiting'}
		<div class="flex flex-wrap items-end gap-3">
			<p>
				{lobby.owner === lobby.playerId
					? 'Choose a difficulty and start the game.'
					: 'Waiting for the host to start the game.'}
			</p>
			{#if lobby.owner === lobby.playerId}
				<label class="flex flex-col gap-1">
					Difficulty
					<select bind:value={difficulty} disabled={pendingAction !== null}>
						<option value="Test">Test</option>
						<option value="Easy">Easy</option>
						<option value="Medium">Medium</option>
						<option value="Hard">Hard</option>
					</select>
				</label>
				<button onclick={startGame} disabled={pendingAction !== null}>Start game</button>
			{/if}
		</div>
	{:else if game}
		<div class="flex flex-col gap-2">
			{#if typeof matchState === 'object'}
				<p>
					{currentPlayerId === lobby.playerId
						? 'Your turn — left-click to reveal; right-click to flag.'
						: `Waiting for ${currentPlayerId}.`}
				</p>
			{:else if matchState === 'Won'}
				<p>Game won.</p>
			{:else}
				<p>No players remain.</p>
			{/if}
			{#if localPlayerState === 'Eliminated'}
				<p>You have been eliminated.</p>
			{/if}
		</div>

		<div
			class="grid w-fit overflow-hidden border border-current"
			style:grid-template-columns={`repeat(${boardColumns}, minmax(2rem, 1fr))`}
			aria-label="Minefield"
		>
			{#each game.board as row, y (y)}
				{#each row as cell, x (`${x}-${y}`)}
					<button
						class:bg-slate-800={cell === 'HiddenCell' || cell === 'FlaggedCell'}
						class:bg-slate-100={typeof cell === 'object'}
						class:bg-red-200={cell === 'MinedCell'}
						class:cursor-not-allowed={!canAct}
						class:text-white={cell === 'HiddenCell' || cell === 'FlaggedCell'}
						class="flex size-8 items-center justify-center border border-current/30 text-sm"
						type="button"
						disabled={!canAct || cell === 'MinedCell' || typeof cell === 'object'}
						aria-label={`${describeCell(cell)} at column ${x + 1}, row ${y + 1}`}
						onclick={() => revealCell(cell, x, y)}
						oncontextmenu={(event) => {
							event.preventDefault();
							flagCell(x, y);
						}}>{cellContent(cell)}</button
					>
				{/each}
			{/each}
		</div>
	{:else}
		<p>Waiting for the game board.</p>
	{/if}

	{#if actionError !== null}
		<p class="text-red-600" role="alert">{actionError}</p>
	{/if}
</section>
