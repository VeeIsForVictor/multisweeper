import * as v from 'valibot';
import { ClientMessage, ServerMessage } from './protocol';
import { type Logger } from 'pino';

export class State {
    #ws: WebSocket
    #logger: Logger

    public constructor(ws: WebSocket) {
        this.#ws = ws;

        this.#ws.addEventListener(
            'message',
            (event) => {
                const message = this.receiveGameMessage(event.data);
                this.handleGameMessage(message);
            }
        )
    }

    sendGameMessage = (message: ClientMessage) => {
        const parsedMessage = v.parse(ClientMessage, message);

        this.#ws.send(JSON.stringify(parsedMessage));
    }

    receiveGameMessage = (message: unknown) => {
        return v.parse(ServerMessage, message);
    }

    handleGameMessage = (message: ServerMessage) => {

    }
}