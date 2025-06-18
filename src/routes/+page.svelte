<script lang="ts">
	import { onMount } from "svelte";
	import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
	import { invoke } from "@tauri-apps/api/core";

	let input_text = $state("");
	let window: WebviewWindow | null = $state(null);

	let input_width = $derived(Math.max(1, input_text.length));

	async function onKeyDown(event: KeyboardEvent) {
		if (event.key !== "Enter") return;

		await invoke("paste_into_previous_app", { text: input_text });
		input_text = "";
		// await invoke("hide_window");
	}

	function createNewWindow() {
		if (window) {
			window.close();
		}
		window = new WebviewWindow("new-window", {
			url: "https://tauri.app",
			width: input_width,
			resizable: true,
			maximizable: false,
			decorations: false,
			shadow: true,
			alwaysOnTop: true,
			focus: true,
		});
	}

	onMount(() => {
		document.getElementById("input-field")?.focus();
		createNewWindow();
	});
</script>

<main class="bg-gray-400 rounded-4xl p-4">
	<input
		class="pl-2 appearance-none w-full focus:outline-none"
		id="input-field"
		type="text"
		placeholder="Enter text..."
		onkeydown={onKeyDown}
		bind:value={input_text}
	/>
</main>
