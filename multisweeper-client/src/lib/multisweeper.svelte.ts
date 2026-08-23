import * as v from 'valibot';
import { ClientMessage, ServerMessage } from './protocol';

export class State {
    #ws: WebSocket

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