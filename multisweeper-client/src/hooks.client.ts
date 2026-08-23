import { SERVER_URL } from "$lib/env";
import { State } from "$lib/multisweeper.svelte";
import { type Handle } from "@sveltejs/kit";
import { pino } from 'pino';

const server = new WebSocket(SERVER_URL);
const logger = pino({
    transport: {
        target: 'pino-pretty',
        options: { colorize: true }
    }
});

process.on(
    'beforeExit',
    () => server.close()
);

const state = new State(server, logger.child({
    'target': 'game-state'
}));

export const handle: Handle = async ({ event, resolve }) => {
    const requestLogger = logger.child({
        'target': 'request'
    })
    event.locals = { state, logger: requestLogger };
    return await resolve(event);
};