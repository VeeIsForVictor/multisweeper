import * as v from 'valibot';

import { ClientDifficulty, ClientGameAction, MessageId, RoomCode, Uint8 } from '../shared';

export const ConnectionPing = v.object({
	type: v.literal('connection.ping'),
});
export type ConnectionPing = v.InferOutput<typeof ConnectionPing>;

export const RoomsList = v.object({
	type: v.literal('rooms.list'),
});
export type RoomsList = v.InferOutput<typeof RoomsList>;

export const RoomJoin = v.object({
	type: v.literal('room.join'),
	room_code: RoomCode
});
export type RoomJoin = v.InferOutput<typeof RoomJoin>;

export const RoomCreate = v.object({
	type: v.literal('room.create'),
});
export type RoomCreate = v.InferOutput<typeof RoomCreate>;

export const RoomLeave = v.object({
	type: v.literal('room.leave'),
});
export type RoomLeave = v.InferOutput<typeof RoomLeave>;

export const GameStart = v.object({
	type: v.literal('game.start'),
	difficulty: ClientDifficulty
});
export type GameStart = v.InferOutput<typeof GameStart>;

export const GameAction = v.object({
	type: v.literal('game.action'),
	action: ClientGameAction,
	x: Uint8,
	y: Uint8
});
export type GameAction = v.InferOutput<typeof GameAction>;

export const RoomStateGet = v.object({
	type: v.literal('room.state.get'),
});
export type RoomStateGet = v.InferOutput<typeof RoomStateGet>;

export const ClientCommand = v.variant('type', [
	ConnectionPing,
	RoomsList,
	RoomJoin,
	RoomCreate,
	RoomLeave,
	GameStart,
	GameAction,
	RoomStateGet
]);
export type ClientCommand = v.InferOutput<typeof ClientCommand>;

export const ClientMessage = v.variant('type', ClientCommand.options.map(
	variant => v.object({
		...variant.entries,
		message_id: MessageId
	})
))