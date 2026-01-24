import { array, literal, number, object, picklist, string, union, type InferOutput } from "valibot";

type LobbyCode = string;
const LobbyCode = string;

type PlayerId = string;
const PlayerId = string;

const LobbyStatus = picklist([
    'Waiting',
    'Starting'
]);

const LobbyState = object({
    LobbyState: object({
        code: LobbyCode(),
        players: array(PlayerId()),
        host_id: PlayerId(),
        status: LobbyStatus
    })
});
export type LobbyState = InferOutput<typeof LobbyState>;

const GameStarted = literal("GameStarted");
export type GameStarted = InferOutput<typeof GameStarted>;

const GameInfo = object({
    GameInfo: object({
        code: LobbyCode(),
        width: number(),
        height: number(),
        number_of_mines: number(),
        seed: number()
    })
})
export type GameInfo = InferOutput<typeof GameInfo>;

export const ServerMessage = union([
    LobbyState,
    GameStarted,
    GameInfo
]);