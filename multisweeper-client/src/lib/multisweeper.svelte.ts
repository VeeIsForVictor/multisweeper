import * as v from 'valibot';
import { ClientMessage } from './protocol';

export class State {
    #ws: WebSocket

    public constructor(ws: WebSocket) {
        this.#ws = ws;
    }

    sendGameMessage = async (message: ClientMessage) => {
        const parsedMessage = v.parse(ClientMessage, message);

        this.#ws.send(JSON.stringify(parsedMessage));
    }
}