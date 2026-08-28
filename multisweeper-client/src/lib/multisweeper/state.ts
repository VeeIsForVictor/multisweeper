import type { PlayerView } from "$lib/protocol";

export type State = 
    ConnectingState 
    | ReadyState
    | LandingState 
    | CreatingState
    | LobbyState
    | FatalState;

type ConnectingState = { type: 'connecting' };
type ReadyState = { type: 'ready', playerId: string };
type LandingState = { type: 'landed', playerId: string, lobbies: string[] }
type CreatingState = { type: 'creating', playerId: string };
type LobbyState = { type: 'lobby', playerId: string, roomId: string, players: PlayerView[] }
type FatalState = { type: 'fatal', message: string }