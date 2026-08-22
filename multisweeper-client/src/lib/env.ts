import { strict } from 'assert';
import * as env from '$env/static/public';

strict(env.PUBLIC_SERVER_URL !== '', 'PUBLIC_MULTISWEEPER_SERVER is not defined in .env');

export const SERVER_URL = env.PUBLIC_SERVER_URL;
