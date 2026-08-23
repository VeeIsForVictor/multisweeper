import * as v from 'valibot';
import { ClientMessage, ServerMessage } from './protocol';
import { type Logger } from 'pino';
import { strict } from 'assert';

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
            }
        )

        this.#logger = logger;
    }

    sendGameMessage = (message: ClientMessage) => {
        const parsedMessage = v.parse(ClientMessage, message);

        this.#ws.send(JSON.stringify(parsedMessage));
    }

    receiveGameMessage = (message: unknown) => {
        return v.parse(ServerMessage, message);
    }

    handleGameMessage = (message: ServerMessage) => {
        switch (message.type) {
            case 'connection.ready':
                strict(this.status === 'connecting');
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