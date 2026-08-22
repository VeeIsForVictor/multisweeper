import { strict } from 'assert';
import * as env from '$env/static/public';

strict(env.PUBLIC_MULTISWEEPER_SERVER !== '', 'PUBLIC_MULTISWEEPER_SERVER is not defined in .env');

export const MULTISWEEPER_SERVER = env.PUBLIC_MULTISWEEPER_SERVER;
