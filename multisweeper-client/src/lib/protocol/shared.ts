import * as v from 'valibot';

export const MessageId = v.string();
export type MessageId = v.InferOutput<typeof MessageId>;

export const PlayerId = v.string();
export type PlayerId = v.InferOutput<typeof PlayerId>;

export const RoomCode = v.string();
export type RoomCode = v.InferOutput<typeof RoomCode>;

export const Uint8 = v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(255));
export type Uint8 = v.InferOutput<typeof Uint8>;

export const CellView = v.union([
	v.picklist(['HiddenCell', 'FlaggedCell', 'MinedCell']),
	v.strictObject({
		VisibleCell: Uint8
	})
]);
export type CellView = v.InferOutput<typeof CellView>;

export const ClientDifficulty = v.picklist(['Test', 'Easy', 'Medium', 'Hard']);
export type ClientDifficulty = v.InferOutput<typeof ClientDifficulty>;

export const ClientGameAction = v.picklist(['reveal', 'flag']);
export type ClientGameAction = v.InferOutput<typeof ClientGameAction>;

export const ErrorCode = v.picklist([
	'room_already_joined',
	'no_room_joined',
	'room_dropped',
	'room_not_found',
	'not_room_owner',
	'game_already_started',
	'game_not_started',
	'game_ended',
	'player_is_spectating',
	'player_eliminated',
	'not_current_player',
	'no_players_remaining',
	'player_not_found',
	'game_error',
	'room_unavailable',
	'invalid_message',
	'duplicate_message_id'
]);
export type ErrorCode = v.InferOutput<typeof ErrorCode>;

export const ClientError = v.object({
	code: ErrorCode,
	message: v.string()
});
export type ClientError = v.InferOutput<typeof ClientError>;

export const GameActionResult = v.picklist(['Applied', 'Stalled', 'Started', 'Eliminated', 'Won']);
export type GameActionResult = v.InferOutput<typeof GameActionResult>;

export const GameStatus = v.picklist(['Won', 'NoWinner', 'Playing']);
export type GameStatus = v.InferOutput<typeof GameStatus>;

export const GameSnapshot = v.object({
	status: GameStatus,
	action_result: GameActionResult,
	board: v.array(v.array(CellView))
});
export type GameSnapshot = v.InferOutput<typeof GameSnapshot>;

const PlayingMatchState = v.strictObject({
	Playing: v.object({
		current_player: PlayerId,
		last_player: v.nullish(PlayerId)
	})
});

export const MatchState = v.union([
	v.picklist(['Waiting', 'Won', 'NoWinner']),
	PlayingMatchState
]);
export type MatchState = v.InferOutput<typeof MatchState>;

export const MatchView = v.object({
	state: MatchState,
	game: v.nullish(GameSnapshot)
});
export type MatchView = v.InferOutput<typeof MatchView>;

export const PlayerState = v.picklist(['Spectator', 'Playing', 'Eliminated']);
export type PlayerState = v.InferOutput<typeof PlayerState>;

export const PlayerView = v.object({
	id: PlayerId,
	state: PlayerState
});
export type PlayerView = v.InferOutput<typeof PlayerView>;
