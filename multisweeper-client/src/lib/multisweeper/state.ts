import type { MatchView, PlayerId, PlayerView, RoomCode, ServerMessage } from "$lib/protocol";

export type State = 
    ConnectingState 
    | ReadyState
    | LandingState 
    | LobbyState
    | FatalState;

export type ConnectingState = { type: 'connecting' };
export type ReadyState = { type: 'ready', playerId: PlayerId };
export type LandingState = { type: 'landed', playerId: PlayerId, lobbies: RoomCode[] };
export type LobbyState = {
    type: 'lobby';
    playerId: PlayerId;
    roomId: RoomCode;
    lobbies: RoomCode[];
    roomsLoaded: boolean;
    owner: PlayerId | null | undefined;
    players: PlayerView[];
    game: MatchView;
};
export type FatalState = { type: 'fatal', message: string };

export type IdentifiedState = Exclude<State, ConnectingState | FatalState>;

export function reduceState(state: State, message: ServerMessage): State {
    switch (message.type) {
        case 'connection.ready':
            if (state.type !== 'connecting') {
                throw new Error('received connection.ready after successful connection');
            }
            return {
                type: 'ready',
                playerId: message.player_id
            };

        case 'rooms.listed':
            if (state.type === 'connecting' || state.type === 'fatal') {
                throw new Error('received room listing before connection.ready');
            }
            if (state.type === 'lobby') {
                return {
                    ...state,
                    lobbies: message.rooms,
                    roomsLoaded: true
                };
            }
            return {
                type: 'landed',
                playerId: state.playerId,
                lobbies: message.rooms
            };

        case 'room.state': {
            if (state.type === 'connecting' || state.type === 'fatal') {
                throw new Error('received room state before connection.ready');
            }
            const lobbies = state.type === 'landed' || state.type === 'lobby'
                ? state.lobbies
                : [];
            const roomsLoaded = state.type === 'landed' || (state.type === 'lobby' && state.roomsLoaded);
            return {
                type: 'lobby',
                playerId: state.playerId,
                roomId: message.code,
                lobbies,
                roomsLoaded,
                owner: message.owner,
                players: message.players,
                game: message.game
            };
        }

        case 'room.removed':
            if (state.type !== 'lobby') {
                throw new Error('received room removal outside of room');
            }
            if (!state.roomsLoaded) {
                return {
                    type: 'ready',
                    playerId: state.playerId
                };
            }
            return {
                type: 'landed',
                playerId: state.playerId,
                lobbies: state.lobbies
            };

        case 'connection.pong':
        case 'command.rejected':
        case 'game.started':
            return state;
    }
}
