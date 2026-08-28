import { SERVER_URL } from "$lib/env";
import { Multisweeper } from "$lib/multisweeper/multisweeper.svelte";
import { type LayoutLoad } from './$types'
import { pino } from 'pino';

export const ssr = false;

const server = new WebSocket(SERVER_URL);
const logger = pino({
    transport: {
        target: 'pino-pretty',
        options: { colorize: true }
    }
});

const ms = new Multisweeper(server, logger.child({
    'target': 'game-state'
}));

export const load: LayoutLoad = async () => {
    return { ms, logger };    
};
