<script lang="ts">
	import { onMount } from "svelte";
	import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
	import { invoke } from "@tauri-apps/api/core";

	let input_text = $state("");
	let window: WebviewWindow | null = $state(null);
	let load_output = $state(false);

	let input_width = $derived(Math.max(1, input_text.length));

	async function onKeyDown(event: KeyboardEvent) {
		if (event.key !== "Enter") return;
		load_output = true;

		await invoke("paste_into_previous_app", { text: input_text });
		input_text = "";
		load_output = false;
		// await invoke("hide_window");
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
		createNewWindow();
	});
</script>

<main class="transparent h-screen">
	<div class="bg-white rounded-l p-2 {load_output ? 'animate-pulse' : ''}">
		<input
			class="pl-1 appearance-none w-full focus:outline-none "
			id="input-field"
			type="text"
			placeholder="Just write"
			onkeydown={onKeyDown}
			bind:value={input_text}
		/>
	</div>
</main>
