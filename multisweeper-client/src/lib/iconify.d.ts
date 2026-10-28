declare module 'virtual:iconify/*' {
	import type { Component } from 'svelte';
	import type { SVGAttributes } from 'svelte/elements';

	const icon: Component<SVGAttributes<SVGSVGElement>>;
	export default icon;
}
