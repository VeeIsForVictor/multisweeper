import { SERVER_URL } from "$lib/env";
import { Multisweeper } from "$lib/multisweeper/multisweeper.svelte";
import { type LayoutLoad } from './$types'
import { pino } from 'pino';

export const ssr = false;

const logger = pino({
    transport: {
        target: 'pino-pretty',
        options: { colorize: true }
    }
});

const ms = new Multisweeper(SERVER_URL, logger.child({
    'target': 'game-state'
}));

export const load: LayoutLoad = async () => {
    return { ms, logger };    
};
