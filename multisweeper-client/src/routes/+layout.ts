import { SERVER_URL } from "$lib/env";
import { State } from "$lib/multisweeper.svelte";
import { type LayoutLoad } from './$types'
import { pino } from 'pino';

const server = new WebSocket(SERVER_URL);
const logger = pino({
    transport: {
        target: 'pino-pretty',
        options: { colorize: true }
    }
});

const state = new State(server, logger.child({
    'target': 'game-state'
}));

export const load: LayoutLoad = async () => {
    return { state, logger };    
};