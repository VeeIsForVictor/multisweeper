import * as v from 'valibot';
import { ClientCommand, ClientMessage, ServerMessage } from '../protocol';
import { type Logger } from 'pino';
import type { State } from './state';

export class Multisweeper {
    #ws: WebSocket
    #logger: Logger

    state = $state<State>({ type: "connecting" });

    public constructor(ws: WebSocket, logger: Logger) {
        this.#ws = ws;

        this.#ws.addEventListener(
            'message',
            (event) => {
                const message = this.receiveGameMessage(event.data);
                this.handleGameMessage(message);
                return true;
            }
        )

        this.#logger = logger;
    }

    sendGameMessage = (message: ClientCommand) => {
        const transportMessage = {
            ...message,
            message_id: crypto.randomUUID()
        }

        const parsedMessage = v.parse(ClientMessage, transportMessage);
        this.#logger.info({
            'message.id': parsedMessage.message_id,
            'message.type': parsedMessage.type
        }, 'sending message'
        )

        this.#ws.send(JSON.stringify(parsedMessage));
    }

    receiveGameMessage = (message: string) => {
        return v.parse(ServerMessage, JSON.parse(message));
    }

    handleGameMessage = (message: ServerMessage) => {
        this.#assert(this.state.type !== 'fatal')
        const messageLogger = this.#logger.child({
            'message.id': message.message_id,
            'message.type': message.type
        })
        messageLogger.info('handling message')
        switch (message.type) {
            case 'connection.ready':
                console.assert(this.state.type === 'connecting');
                this.state = {
                    type: "no-lobby",
                    playerId: message.player_id,
                    lobbies: []
                }
                this.sendGameMessage({
                    type: "rooms.list"
                })
                this.#logger = this.#logger.child({
                    'player.id': message.player_id
                })
                break;
            case 'rooms.listed':
                this.#assert(this.state.type === 'no-lobby');
                this.state = {
                    type: "no-lobby",
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
}