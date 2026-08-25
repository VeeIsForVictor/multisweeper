export type State = 
    ConnectingState 
    | ReadyState
    | LandingState 
    | FatalState;

type ConnectingState = { type: 'connecting' };
type ReadyState = { type: 'ready', playerId: string };
type LandingState = { type: 'landed', playerId: string, lobbies: string[] }
type FatalState = { type: 'fatal', message: string }