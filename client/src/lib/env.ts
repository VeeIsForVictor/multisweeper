import { env } from "$env/dynamic/public";
import strict from "node:assert/strict";

strict(typeof env.PUBLIC_SERVER_URL != 'undefined')

export const SERVER_URL = env.PUBLIC_SERVER_URL;