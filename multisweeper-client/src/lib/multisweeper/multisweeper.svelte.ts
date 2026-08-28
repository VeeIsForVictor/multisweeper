import * as v from 'valibot';
import { ClientCommand, ClientMessage, ServerMessage } from '../protocol';
import { type Logger } from 'pino';
import type { State } from './state';
import { SvelteMap } from 'svelte/reactivity';

type MessageResolve = (value: ServerMessage | PromiseLike<ServerMessage>) => void;

export class Multisweeper {
    #ws: WebSocket
    #logger: Logger
    #promiseTable: Map<string, MessageResolve> = new SvelteMap<string, MessageResolve>();

    state = $state<State>({ type: "connecting" });

    public constructor(ws: WebSocket, logger: Logger) {
        this.#ws = ws;

        this.#ws.addEventListener(
            'message',
            (event) => {
                const message = this.#receiveGameMessage(event.data);
                this.#handleGameMessage(message);
                return true;
            }
        )

        this.#logger = logger;
    }

    #sendGameMessage: (message: ClientCommand) => Promise<ServerMessage> = async (message: ClientCommand) => {
        this.#assert(this.state.type !== 'connecting', "client is not ready to send messages")

        const messageId = crypto.randomUUID();
        const transportMessage = {
            ...message,
            message_id: messageId
        }

        const parsedMessage = v.parse(ClientMessage, transportMessage);
        this.#logger.info({
            'message.id': parsedMessage.message_id,
            'message.type': parsedMessage.type
        }, 'sending message'
        )

        const promise = new Promise<ServerMessage>((resolve) => {
            this.#promiseTable.set(messageId, resolve)
        });
        try {
            this.#ws.send(JSON.stringify(parsedMessage));
        } catch (e) {
            this.#promiseTable.delete(messageId);
            throw e;
        }

        return promise;
    }

    #receiveGameMessage = (message: string) => {
        return v.parse(ServerMessage, JSON.parse(message));
    }

    #handleGameMessage = (message: ServerMessage) => {
        this.#assert(this.state.type !== 'fatal');

        if (message.type !== 'connection.ready' && typeof message.correlation_id === 'string') {
            const correlatedPromise = this.#promiseTable.get(message.correlation_id);
            if (typeof correlatedPromise !== 'undefined') {
                correlatedPromise(message);
                this.#promiseTable.delete(message.correlation_id);
            }
        }
        
        const messageLogger = this.#logger.child({
            'message.id': message.message_id,
            'message.type': message.type
        })
        messageLogger.info('handling message')
        switch (message.type) {
            case 'connection.ready':
                this.#assert(this.state.type === 'connecting', 'received connection.ready after successful connection');
                this.state = {
                    type: "ready",
                    playerId: message.player_id,
                }
                this.#logger = this.#logger.child({
                    'player.id': message.player_id
                })
                break;
            case 'rooms.listed':
                this.#assert(this.state.type !== 'connecting', 'received room listing while connecting');
                this.state = {
                    type: "landed",
                    playerId: this.state.playerId,
                    lobbies: message.rooms
                }
                break;
            case 'room.state':
                this.#assert(this.state.type !== 'connecting', 'received room state while connecting');
                this.state = {
                    type: "lobby",
                    playerId: this.state.playerId,
                    roomId: message.code,
                    players: message.players
                }
                break;
            case 'room.removed':
                this.#assert(this.state.type === 'lobby', 'received room kick outside of room');
                this.state = {
                    type: 'ready',
                    playerId: this.state.playerId
                }
        }
    }

    #assert(condition: boolean, errorMessage: string = 'unexpected error'): asserts condition {
        if (condition) return;
        else{ 
            this.#logger.error({
                message: errorMessage
            }, 'game invariant violated');
            this.#ws.close();
            this.state = {
                type: 'fatal',
                message: errorMessage
            }
            throw Error(errorMessage);
        }    
    }

    queryRooms = async () => {
        const response = await this.#sendGameMessage({
            type: "rooms.list"
        })

        this.#assert(response.type === 'rooms.listed', 'unexpected response to room query');
        return response.rooms;
    }

    joinRoom = async (roomId: string) => {
        const response = await this.#sendGameMessage({
            type: 'room.join',
            room_code: roomId
        });

        this.#assert(response.type === 'room.state' || response.type === 'command.rejected');

        return new Promise<State>((resolve, reject) => {
            if (response.type === 'room.state')
                return resolve(this.state);
            else 
                return reject(response.error);
        })
    }

    quit = async () => {
        await this.leaveRoom();
        this.#ws.close();
        this.#logger.warn("terminating player instance")
    }

    leaveRoom = async () => {
        return await this.#sendGameMessage({
            type: "room.leave"
        });
    }

    createRoom = async () => {
        this.#assert(this.roomId === null && this.playerId !== null);
        this.state = {
            type: 'creating',
            playerId: this.playerId
        }
        return this.#sendGameMessage({
            type: 'room.create'
        }).then((message) => {
            this.#assert(message.type === 'room.state');
            return message.code;
        })
    }

    public get playerId() : string | null {
        this.#assert(this.state.type !== 'fatal');
        return this.state.type === 'connecting'
            ? null
            : this.state.playerId
    }

    public get roomId() : string | null {
        return this.state.type === 'lobby'
            ? this.state.roomId
            : null
    }
    
}