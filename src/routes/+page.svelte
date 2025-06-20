<script lang="ts">
	import { onMount } from "svelte";
	import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
	import { invoke } from "@tauri-apps/api/core";

	let input_text = $state("");
	let window: WebviewWindow | null = $state(null);
	let load_output = $state(false);

	let input_width = $derived(Math.max(1, input_text.length));
	let prev_window_name: String = $state("");

	async function onKeyDown(event: KeyboardEvent) {
		getPrevWindowName().then((name) => {
			prev_window_name =
				name.length >= 10 ? name.substring(0, 10) + "..." : name;
		});
		if (event.key !== "Enter") return;
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

	function createNewWindow() {
		if (window) {
			window.close();
		}
		window = new WebviewWindow("new-window", {
			url: "https://tauri.app",
			resizable: false,
			maximizable: false,
			shadow: true,
			alwaysOnTop: true,
			focus: true,
		});
	}

	onMount(() => {
		document.getElementById("input-field")?.focus();
		setTimeout(() => {
			document.getElementById("input-field")?.focus();
		}, 100); // Focus after a short delay to ensure the input is ready
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
