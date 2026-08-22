import * as v from 'valibot';

import {
	ClientError,
	MatchView,
	MessageId,
	PlayerId,
	PlayerView,
	RoomCode
} from '../shared';

export const ConnectionReady = v.object({
	type: v.literal('connection.ready'),
	message_id: MessageId,
	player_id: PlayerId
});
export type ConnectionReady = v.InferOutput<typeof ConnectionReady>;

export const ConnectionPong = v.object({
	type: v.literal('connection.pong'),
	message_id: MessageId,
	correlation_id: MessageId
});
export type ConnectionPong = v.InferOutput<typeof ConnectionPong>;

export const RoomsListed = v.object({
	type: v.literal('rooms.listed'),
	message_id: MessageId,
	correlation_id: MessageId,
	rooms: v.array(RoomCode)
});
export type RoomsListed = v.InferOutput<typeof RoomsListed>;

export const RoomState = v.object({
	type: v.literal('room.state'),
	message_id: MessageId,
	correlation_id: v.nullish(MessageId),
	code: RoomCode,
	owner: v.nullish(PlayerId),
	players: v.array(PlayerView),
	game: MatchView
});
export type RoomState = v.InferOutput<typeof RoomState>;

export const RoomRemoved = v.object({
	type: v.literal('room.removed'),
	message_id: MessageId,
	correlation_id: v.nullish(MessageId),
	reason: v.string()
});
export type RoomRemoved = v.InferOutput<typeof RoomRemoved>;

export const CommandRejected = v.object({
	type: v.literal('command.rejected'),
	message_id: MessageId,
	correlation_id: v.nullish(MessageId),
	error: ClientError
});
export type CommandRejected = v.InferOutput<typeof CommandRejected>;

export const GameStarted = v.object({
	type: v.literal('game.started'),
	message_id: MessageId,
	correlation_id: v.nullish(MessageId)
});
export type GameStarted = v.InferOutput<typeof GameStarted>;

export const ServerMessage = v.variant('type', [
	ConnectionReady,
	ConnectionPong,
	RoomsListed,
	RoomState,
	RoomRemoved,
	CommandRejected,
	GameStarted
]);
export type ServerMessage = v.InferOutput<typeof ServerMessage>;
