export type State = 
    ConnectingState 
    | NoLobbyState 
    | FatalState;

type ConnectingState = { type: 'connecting' };
type NoLobbyState = { type: 'no-lobby', playerId: string, lobbies: string[] }
type FatalState = { type: 'fatal', message: string }