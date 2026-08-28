import * as v from 'valibot';
import { ClientCommand, ClientMessage, ServerMessage } from '../protocol';
import type { ClientError, RoomCode } from '../protocol';
import { type Logger } from 'pino';
import { reduceState, type LobbyState, type State } from './state';

const REQUEST_TIMEOUT_MS = 10_000;

type PendingRequest = {
    resolve: (value: ServerMessage | PromiseLike<ServerMessage>) => void;
    reject: (reason?: unknown) => void;
    timeout: ReturnType<typeof setTimeout>;
};

export class CommandRejectedError extends Error {
    public readonly code: ClientError['code'];

    public constructor(error: ClientError) {
        super(error.message);
        this.name = 'CommandRejectedError';
        this.code = error.code;
    }
}

export class Multisweeper {
    #ws: WebSocket
    #logger: Logger
    // eslint-disable-next-line svelte/prefer-svelte-reactivity
    #pendingRequests = new Map<string, PendingRequest>();
    #roomOperation: Promise<void> = Promise.resolve();
    #joinRequest: { roomId: RoomCode; promise: Promise<LobbyState> } | null = null;
    #listRequest: Promise<RoomCode[]> | null = null;
    #createRequest: Promise<RoomCode> | null = null;
    #quitRequest: Promise<void> | null = null;
    #disposed = false;
    #fatal = false;
    #resolveReady: () => void = () => undefined;
    #rejectReady: (reason?: unknown) => void = () => undefined;
    #readyPromise: Promise<void>;
    #state = $state<State>({ type: 'connecting' });

    public get state(): State {
        return this.#state;
    }

    public constructor(ws: WebSocket, logger: Logger) {
        this.#ws = ws;
        this.#logger = logger;
        this.#readyPromise = new Promise<void>((resolve, reject) => {
            this.#resolveReady = resolve;
            this.#rejectReady = reject;
        });
        void this.#readyPromise.catch(() => undefined);

        this.#ws.addEventListener('message', (event) => {
            try {
                const message = this.#receiveGameMessage(event.data);
                this.#handleGameMessage(message);
            } catch (error) {
                this.#enterFatal(error);
            }
        });

        this.#ws.addEventListener('error', () => {
            this.#enterFatal(new Error('WebSocket error'));
        });

        this.#ws.addEventListener('close', (event) => {
            if (!this.#disposed) {
                this.#enterFatal(new Error(`WebSocket closed (${event.code})`));
            } else {
                this.#rejectPending(new Error('Multisweeper session closed'));
            }
        });
    }

    #sendGameMessage = async (message: ClientCommand): Promise<ServerMessage> => {
        await this.#readyPromise;

        if (this.#disposed) {
            throw new Error('Multisweeper session is closed');
        }

        if (this.#ws.readyState !== WebSocket.OPEN) {
            throw new Error('WebSocket is not open');
        }

        const messageId = crypto.randomUUID();
        const transportMessage = {
            ...message,
            message_id: messageId
        }

        const parsedMessage = v.parse(ClientMessage, transportMessage);
        this.#logger.info({
            'message.id': parsedMessage.message_id,
            'message.type': parsedMessage.type
        }, 'sending message'
        )

        const promise = new Promise<ServerMessage>((resolve, reject) => {
            const timeout = setTimeout(() => {
                this.#pendingRequests.delete(messageId);
                reject(new Error(`Timed out waiting for response to ${parsedMessage.type}`));
            }, REQUEST_TIMEOUT_MS);

            this.#pendingRequests.set(messageId, { resolve, reject, timeout });
        });
        try {
            this.#ws.send(JSON.stringify(parsedMessage));
        } catch (e) {
            const pending = this.#pendingRequests.get(messageId);
            if (pending) clearTimeout(pending.timeout);
            this.#pendingRequests.delete(messageId);
            throw e;
        }

        return promise;
    }

    #receiveGameMessage = (message: unknown) => {
        if (typeof message !== 'string') {
            throw new Error('expected a text WebSocket message');
        }

        return v.parse(ServerMessage, JSON.parse(message));
    }

    #handleGameMessage = (message: ServerMessage) => {
        this.#assert(!this.#fatal, 'received a message after a fatal session error');

        const messageLogger = this.#logger.child({
            'message.id': message.message_id,
            'message.type': message.type
        })
        messageLogger.info('handling message')

        this.#state = reduceState(this.#state, message);

        if (message.type === 'connection.ready') {
            this.#logger = this.#logger.child({
                'player.id': message.player_id
            })
            this.#resolveReady();
            void this.queryRooms().catch((error) => {
                this.#logger.warn({ error }, 'initial room query failed');
            });
        }

        if (message.type !== 'connection.ready' && typeof message.correlation_id === 'string') {
            const pending = this.#pendingRequests.get(message.correlation_id);
            if (pending) {
                clearTimeout(pending.timeout);
                this.#pendingRequests.delete(message.correlation_id);
                if (message.type === 'command.rejected') {
                    pending.reject(new CommandRejectedError(message.error));
                } else {
                    pending.resolve(message);
                }
            }
        }
    }

    #assert(condition: boolean, errorMessage: string = 'unexpected error'): asserts condition {
        if (!condition) throw new Error(errorMessage);
    }

    #enterFatal = (cause: unknown) => {
        if (this.#fatal) return;

        this.#fatal = true;
        const error = cause instanceof Error ? cause : new Error(String(cause));
        this.#logger.error({ error }, 'game invariant or transport failure');
        this.#state = {
            type: 'fatal',
            message: error.message
        };
        this.#rejectReady(error);
        this.#rejectPending(error);

        try {
            this.#ws.close();
        } catch (closeError) {
            this.#logger.warn({ error: closeError }, 'failed to close WebSocket after fatal error');
        }
    };

    #rejectPending = (reason: unknown) => {
        for (const [messageId, pending] of this.#pendingRequests) {
            clearTimeout(pending.timeout);
            pending.reject(reason);
            this.#pendingRequests.delete(messageId);
        }
    };

    #enqueueRoomOperation = <T>(operation: () => Promise<T>): Promise<T> => {
        const next = this.#roomOperation.then(operation);
        this.#roomOperation = next.then(() => undefined, () => undefined);
        return next;
    };

    #protocolFailure = (message: string): never => {
        const error = new Error(message);
        this.#enterFatal(error);
        throw error;
    }

    queryRooms = (): Promise<RoomCode[]> => {
        if (this.#listRequest) return this.#listRequest;

        const request = this.#sendGameMessage({ type: 'rooms.list' }).then((response) => {
            if (response.type !== 'rooms.listed') {
                return this.#protocolFailure('unexpected response to room query');
            }
            return response.rooms;
        });

        this.#listRequest = request;
        void request.then(
            () => { if (this.#listRequest === request) this.#listRequest = null; },
            () => { if (this.#listRequest === request) this.#listRequest = null; }
        );
        return request;
    }

    joinRoom = (roomId: RoomCode): Promise<LobbyState> => {
        if (this.#joinRequest?.roomId === roomId) return this.#joinRequest.promise;

        const promise = this.#enqueueRoomOperation(async () => {
            await this.#readyPromise;

            if (this.#state.type === 'lobby' && this.#state.roomId === roomId) {
                return this.#state;
            }

            if (this.#state.type === 'lobby') {
                await this.#leaveRoomNow();
            }

            const response = await this.#sendGameMessage({
                type: 'room.join',
                room_code: roomId
            });

            if (response.type !== 'room.state') {
                return this.#protocolFailure('unexpected response to room join');
            }

            const state = this.#state;
            if (state.type !== 'lobby' || state.roomId !== roomId) {
                return this.#protocolFailure('room join response did not update client state');
            }
            return state;
        });

        this.#joinRequest = { roomId, promise };
        void promise.then(
            () => { if (this.#joinRequest?.promise === promise) this.#joinRequest = null; },
            () => { if (this.#joinRequest?.promise === promise) this.#joinRequest = null; }
        );
        return promise;
    };

    #leaveRoomNow = async (): Promise<void> => {
        if (this.#state.type !== 'lobby') return;

        const response = await this.#sendGameMessage({ type: 'room.leave' });
        if (response.type !== 'room.removed') {
            return this.#protocolFailure('unexpected response to room leave');
        }
    };

    leaveRoom = (): Promise<void> => this.#enqueueRoomOperation(() => this.#leaveRoomNow());

    createRoom = async (): Promise<RoomCode> => {
        if (this.#createRequest) return this.#createRequest;

        const request = this.#createRoom();
        this.#createRequest = request;
        void request.then(
            () => { if (this.#createRequest === request) this.#createRequest = null; },
            () => { if (this.#createRequest === request) this.#createRequest = null; }
        );
        return request;
    };

    #createRoom = async (): Promise<RoomCode> => {
        await this.#readyPromise;

        if (this.#state.type === 'lobby') {
            throw new Error('cannot create a room while already in a room');
        }

        const response = await this.#sendGameMessage({ type: 'room.create' });
        if (response.type !== 'room.state') {
            return this.#protocolFailure('unexpected response to room creation');
        }
        return response.code;
    }

    quit = (): Promise<void> => {
        if (this.#quitRequest) return this.#quitRequest;

        const request = this.#quit();
        this.#quitRequest = request;
        return request;
    };

    #quit = async (): Promise<void> => {
        if (this.#disposed) return;

        try {
            await this.leaveRoom();
        } catch (error) {
            this.#logger.warn({ error }, 'failed to leave room while terminating player instance');
        } finally {
            this.#disposed = true;
            this.#rejectReady(new Error('Multisweeper session closed'));
            this.#rejectPending(new Error('Multisweeper session closed'));
            try {
                this.#ws.close();
            } catch (error) {
                this.#logger.warn({ error }, 'failed to close WebSocket while terminating player instance');
            }
            this.#logger.warn('terminating player instance');
        }
    }

}
