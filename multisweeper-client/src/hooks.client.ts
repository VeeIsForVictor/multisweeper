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

const state = new State(server);

export const handle: Handle = async ({ event, resolve }) => {
    event.locals = { state, logger };
    return await resolve(event);
};