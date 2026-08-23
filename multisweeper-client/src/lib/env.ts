import * as env from '$env/static/public';

console.assert(env.PUBLIC_SERVER_URL !== '', 'PUBLIC_MULTISWEEPER_SERVER is not defined in .env');

export const SERVER_URL = env.PUBLIC_SERVER_URL;
