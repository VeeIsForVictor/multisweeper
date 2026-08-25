export type State = ConnectingState | NoLobbyState;

type ConnectingState = { type: 'connecting' };
type NoLobbyState = { type: 'no-lobby', playerId: string, lobbies: string[] }