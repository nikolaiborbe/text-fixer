<script lang="ts">
	import { onMount } from "svelte";
	import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
	import { invoke } from "@tauri-apps/api/core";
	import type { WindowPos } from "../types";
	import AiLogo from "../components/icons/AiLogo.svelte";

	let input_text = $state("");
	let window: WebviewWindow | null = $state(null);
	let load_output = $state(false);

	let prev_window_name: String = $state("");

	async function onKeyDown(event: KeyboardEvent) {
		if (event.key !== "Enter") return;
		load_output = true;
		await invoke("paste_into_previous_app", { text: input_text });
		input_text = "";
		load_output = false;
	}

	async function setPrevWindowName() {
		const length = 13;
		try {
			const prev_name = String(await invoke<string>("get_last_window_name"));
			prev_window_name =
				prev_name.length >= length
					? String(await invoke<string>("get_last_window_name")).slice(
							0,
							length,
						) + "..."
					: prev_name;
		} catch (error) {
			console.error("Error getting previous window name:", error);
			prev_window_name = "Unknown";
		}
	}

	onMount(() => {
		document.getElementById("input-field")?.focus();
	});
</script>

<main class="flex items-center justify-center min-h-screen">
	<div
		class="
		p-[1px] rounded-lg w-full
		{load_output ? 'fancy-border' : 'p-0'} 
		"
	>
		<div class="flex items-center bg-black rounded-lg px-3 py-3">
			<AiLogo />

			<input
				class="flex-1 bg-transparent focus:outline-none text-white px-2"
				placeholder="Just write"
				type="text"
				id="input-field"
				bind:value={input_text}
				onfocus={() => {
					setPrevWindowName();
					document.getElementById("input-field")?.focus();
				}}
				onkeydown={onKeyDown}
			/>

			<p class="text-white/60">{prev_window_name}</p>
		</div>
	</div>
</main>

<style>
	@property --border-angle {
		syntax: "<angle>";
		initial-value: 0deg;
		inherits: false;
	}

	@keyframes spinBorder {
		to {
			--border-angle: 360deg;
		}
	}

	.fancy-border {
		border-radius: 0.5rem;
		border: 1px solid transparent;

		background:
			conic-gradient(
					from var(--border-angle),
					#000000 40%,
					#74558d 65%,
					#e95ad6 70%,
					#74558d 85%,
					#000000 100%
				)
				border-box,
			transparent padding-box;

		--border-angle: 0deg;
		animation: spinBorder 3s cubic-bezier(0.39, 0.575, 0.565, 1) infinite;
	}
</style>
