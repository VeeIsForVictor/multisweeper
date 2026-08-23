import * as v from 'valibot';
import { ClientMessage, ServerMessage } from './protocol';
import { type Logger } from 'pino';

export interface StateView {
    playerId: string,
    roomCode: string | null
}

export class State {
    #ws: WebSocket
    #logger: Logger

    status = $state< 'connecting' | 'ready' >('connecting');
    view = $state.raw< StateView | null >(null)

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

    sendGameMessage = (message: ClientMessage) => {
        const parsedMessage = v.parse(ClientMessage, message);

        this.#ws.send(JSON.stringify(parsedMessage));
    }

    receiveGameMessage = (message: string) => {
        return v.parse(ServerMessage, JSON.parse(message));
    }

    handleGameMessage = (message: ServerMessage) => {
        const messageLogger = this.#logger.child({
            'message.id': message.message_id,
            'message.type': message.type
        })
        messageLogger.info('handling message')
        switch (message.type) {
            case 'connection.ready':
                console.assert(this.status === 'connecting');
                this.view = {
                    playerId: message.player_id,
                    roomCode: null
                }
                this.status = 'ready';
                this.#logger = this.#logger.child({
                    'player.id': message.player_id
                })
        }
    }
}