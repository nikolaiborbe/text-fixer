<script lang="ts">
	import { onMount } from "svelte";
	import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
	import { invoke } from "@tauri-apps/api/core";

	let input_text = $state("");
	let window: WebviewWindow | null = $state(null);
	let load_output = $state(false);
	let window_pos = $state<WindowPos>({ x: 0, y: 0 });

	let prev_window_name: String = $state("");

	interface WindowPos {
		x: number;
		y: number;
	}

	async function onKeyDown(event: KeyboardEvent) {
		if (event.key !== "Enter") return;

		getPrevWindowName().then((name) => {
			prev_window_name =
				name.length >= 10 ? name.substring(0, 10) + "..." : name;
		});
		load_output = true;

		await invoke("paste_into_previous_app", { text: input_text });
		input_text = "";
		load_output = false;
		// await invoke("hide_window");
	}

	async function getPrevWindowName() {
		try {
			return String(await invoke<string>("get_prev_window_name"));
		} catch (error) {
			return "";
		}
	}

	async function getWindowPos(): Promise<WindowPos> {
		const pos = await invoke<WindowPos>("get_prev_window_pos");
		return await pos;
	}

	function createNewWindow() {
		if (window instanceof WebviewWindow) {
		}
		console.log("test");
	}

	onMount(() => {
		document.getElementById("input-field")?.focus();
		setTimeout(() => {
			document.getElementById("input-field")?.focus();
		}, 100); // Focus after a short delay to ensure the input is ready

		getWindowPos()
			.then((pos) => {
				window_pos = pos;
			})
			.catch((e) => {
				console.error("Error getting window position:", e);
			});

		createNewWindow();
		getPrevWindowName();
	});
</script>

<main class="transparent h-screen">
	<div
		class="flex justify-around bg-white rounded-l p-2 {load_output
			? 'animate-pulse'
			: ''}"
	>
		<input
			class="pl-1 appearance-none w-full focus:outline-none flex-1"
			id="input-field"
			type="text"
			placeholder="Just write"
			onload={() => document.getElementById("input-field")?.focus()}
			onkeydown={onKeyDown}
			bind:value={input_text}
		/>
		<p class="pl-2 pr-1 text-gray-500">{prev_window_name}</p>
	</div>
</main>
