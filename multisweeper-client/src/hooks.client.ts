import { SERVER_URL } from "$lib/env";
import { State } from "$lib/multisweeper.svelte";
import { type Handle } from "@sveltejs/kit";

const server = new WebSocket(SERVER_URL);

process.on(
    'beforeExit',
    () => server.close()
)

const state = new State(server);

export const handle: Handle = async ({ event, resolve }) => {
    event.locals.state = state;
    return await resolve(event);
};