// See https://svelte.dev/docs/kit/types#app.d.ts

import type { State } from "$lib/multisweeper.svelte";

// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		interface Locals {
			state: State
		}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

export {};
