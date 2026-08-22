import * as v from 'valibot';

import { ClientDifficulty, ClientGameAction, MessageId, RoomCode, Uint8 } from '../shared';

export const ConnectionPing = v.object({
	type: v.literal('connection.ping'),
	message_id: MessageId
});
export type ConnectionPing = v.InferOutput<typeof ConnectionPing>;

export const RoomsList = v.object({
	type: v.literal('rooms.list'),
	message_id: MessageId
});
export type RoomsList = v.InferOutput<typeof RoomsList>;

export const RoomJoin = v.object({
	type: v.literal('room.join'),
	message_id: MessageId,
	room_code: RoomCode
});
export type RoomJoin = v.InferOutput<typeof RoomJoin>;

export const RoomCreate = v.object({
	type: v.literal('room.create'),
	message_id: MessageId
});
export type RoomCreate = v.InferOutput<typeof RoomCreate>;

export const RoomLeave = v.object({
	type: v.literal('room.leave'),
	message_id: MessageId
});
export type RoomLeave = v.InferOutput<typeof RoomLeave>;

export const GameStart = v.object({
	type: v.literal('game.start'),
	message_id: MessageId,
	difficulty: ClientDifficulty
});
export type GameStart = v.InferOutput<typeof GameStart>;

export const GameAction = v.object({
	type: v.literal('game.action'),
	message_id: MessageId,
	action: ClientGameAction,
	x: Uint8,
	y: Uint8
});
export type GameAction = v.InferOutput<typeof GameAction>;

export const RoomStateGet = v.object({
	type: v.literal('room.state.get'),
	message_id: MessageId
});
export type RoomStateGet = v.InferOutput<typeof RoomStateGet>;

export const ClientMessage = v.variant('type', [
	ConnectionPing,
	RoomsList,
	RoomJoin,
	RoomCreate,
	RoomLeave,
	GameStart,
	GameAction,
	RoomStateGet
]);
export type ClientMessage = v.InferOutput<typeof ClientMessage>;
