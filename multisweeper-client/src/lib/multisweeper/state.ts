export type State = 
    ConnectingState 
    | ReadyState
    | LandingState 
    | LobbyState
    | FatalState;

type ConnectingState = { type: 'connecting' };
type ReadyState = { type: 'ready', playerId: string };
type LandingState = { type: 'landed', playerId: string, lobbies: string[] }
type LobbyState = { type: 'lobby', playerId: string, roomId: string, players: string[] }
type FatalState = { type: 'fatal', message: string }