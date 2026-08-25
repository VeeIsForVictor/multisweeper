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

        this.#ws.send(JSON.stringify(parsedMessage));

        return new Promise((resolve) => {
            this.#promiseTable.set(messageId, resolve)
        });
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
                console.assert(this.state.type === 'connecting');
                this.state = {
                    type: "ready",
                    playerId: message.player_id,
                }
                this.#logger = this.#logger.child({
                    'player.id': message.player_id
                })
                break;
            case 'rooms.listed':
                this.#assert(this.state.type === 'ready');
                this.state = {
                    type: "landed",
                    playerId: this.state.playerId,
                    lobbies: message.rooms
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

        this.#assert(response.type === 'rooms.listed');
        return response.rooms;
    }
}