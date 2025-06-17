#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use reqwest::Client;
use text_fixer_lib::*;
#[cfg(not(target_os = "macos"))]
use tokio::io::ReadHalf; // import from lib.rs
mod window_ops;

use std::sync::{Arc, Mutex};
use tauri::{
  generate_context,
  menu::MenuBuilder,
  tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
  webview::WebviewWindowBuilder,
  AppHandle, Manager, WebviewUrl, WindowEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};
use x_win;

use arboard::Clipboard;

fn main() {
  // Load variables from `.env` at project root so OPENAI_API_KEY is available
  dotenvy::dotenv().ok();
  tauri::Builder::default()
    .plugin(tauri_plugin_clipboard_manager::init())
    .manage(PastePlugin::default())
    .invoke_handler(tauri::generate_handler![
      hide_window,
      paste_into_previous_app,
      improve_text, // new
    ])
    .setup(|app| {
      // Global shortcut we want to register (⌘‑Space)
      let shortcut = Shortcut::new(Some(Modifiers::ALT), Code::Space);

      // Register the global‑shortcut plugin **once** with its handler
      app.handle().plugin(
        tauri_plugin_global_shortcut::Builder::new()
          .with_handler(move |app_handle, s, event| {
            if s == &shortcut && matches!(event.state(), ShortcutState::Pressed) {
              // get current active window
              let plugin_state = app_handle.state::<PastePlugin>();
              if let Ok(w) = x_win::get_active_window() {
                *plugin_state.last_window.lock().unwrap() = Some(w);
              }
              // activate the window with shortcut
              if let Some(window) = app_handle.get_webview_window("main") {
                // Toggle visibility: hide if already visible, else show & focus
                match window.is_visible() {
                  Ok(true) => {
                    let _ = window.hide();
                  }
                  _ => {
                    let _ = window.show();
                    let _ = window.set_focus();
                  }
                }
              } else {
                // Otherwise, create a new main window
                show_or_create_main_window(app_handle);
              }
            }
          })
          .build(),
      )?;

      // Tell the OS we actually want that shortcut
      app.global_shortcut().register(shortcut)?;

      // ── Tray icon & menu ─────────────────────────────────────────────────────
      let tray_menu = MenuBuilder::new(app)
        .text("show", "Show")
        .separator()
        .text("quit", "Quit")
        .build()?;

      let icon = tauri::include_image!("icons/tray/icon.png");

      TrayIconBuilder::with_id("main-tray")
        .icon(icon)
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .build(app)?;

      Ok(())
    })
    .on_tray_icon_event(|app, event| {
      if matches!(
        event,
        TrayIconEvent::Click {
          button: MouseButton::Left,
          ..
        }
      ) {
        show_or_create_main_window(app);
      }
    })
    .on_menu_event(|app, event| match event.id().as_ref() {
      "show" => show_or_create_main_window(app),
      "quit" => std::process::exit(0),
      _ => {}
    })
    .on_window_event(|window, event| {
      if let WindowEvent::CloseRequested { api, .. } = event {
        let _ = window.hide();
        api.prevent_close();
      }
    })
    .run(generate_context!())
    .expect("error while running tauri app");
}

fn show_or_create_main_window<R: tauri::Runtime>(app: &tauri::AppHandle<R>) {
  if let Some(w) = app.get_webview_window("main") {
    let _ = w.show();
    let _ = w.set_focus();
  } else {
    let _ = WebviewWindowBuilder::new(
      app,
      "main",
      WebviewUrl::App("index.html".into()), // Svelte entry‑point
    )
    .title("LiveRewrite")
    .min_inner_size(400.0, 300.0)
    .build();
  }
}

async fn get_openai_response(prompt: &str) -> Result<String, String> {
  let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| "OPENAI_API_KEY not set".to_owned())?;

  let user_msg = Message {
    role: "user",
    content: &prompt,
  };
  let payload = ChatRequest {
    model: "gpt-4o-mini",
    messages: &[user_msg],
  };

  let client = Client::new();
  let res = client
    .post("https://api.openai.com/v1/chat/completions")
    .bearer_auth(api_key)
    .json(&payload)
    .send()
    .await
    .map_err(|e| e.to_string())?;

  if !res.status().is_success() {
    return Err(format!("OpenAI API error: {}", res.status()));
  }

  let chat: ChatResponse = res.json().await.map_err(|e| e.to_string())?;

  let answer = chat
    .choices
    .get(0)
    .map(|c| c.message.content.trim().to_owned())
    .unwrap_or_default();

  Ok(answer)
}

/// Build a prompt that rewrites `text` into clear, everyday language and calls the LLM.
/// Returns the rewritten string or an error string.
async fn improve(text: &str) -> Result<String, String> {
  let prompt = format!(
    "Rewrite the following text so that it reads naturally, with clear and concise sentences. \
    Use everyday vocabulary, keep the original meaning intact, and avoid wording that feels AI‑generated. \
    Around the response put < > for me to format it. Example: wat is the meening of life? Response: <What is the meaning of life?\n\n\"{}\"",
    text.trim()
  );
  get_openai_response(&prompt).await
}

#[tauri::command]
async fn improve_text(text: String) -> Result<String, String> {
  improve(&text).await
}

#[tauri::command]
fn hide_window(app: AppHandle) {
  if let Some(window) = app.get_webview_window("main") {
    let _ = window.hide();
  }
}

#[derive(Default)]
struct PastePlugin {
  last_window: Arc<Mutex<Option<x_win::WindowInfo>>>,
}

#[tauri::command]
async fn paste_into_previous_app(
  text: String,
/*   app: tauri::AppHandle, */
  state: tauri::State<'_, PastePlugin>,
) -> Result<(String, String), String> {
  println!("{}", text);
  let response = improve(&text).await?;

  let last_window = {
    let guard = state.last_window.lock().unwrap();
    guard.clone()
  };

  if let Some(win) = last_window {
    window_ops::activate(&win).map_err(|e| e.to_string())?;
    platform_paste_text(&response)?;
  }

  println!("AI: {}", response);
  Ok((text, response))
}

#[allow(dead_code)]
fn platform_send_paste_shortcut(_: &x_win::WindowInfo) -> Result<(), String> {
  Ok(())
}

use enigo::{
  Direction::{Click, Press, Release},
  Enigo, Key, Keyboard, Settings,
};
/// Copy `text` to the system clipboard and simulate Cmd/Ctrl‑V so it
/// appears in the now‑focused window.
fn platform_paste_text(text: &str) -> Result<(), String> {
  let mut cb = Clipboard::new().map_err(|e| e.to_string())?;
  cb.set_text(text.to_owned()).map_err(|e| e.to_string())?;

  let mut enigo = Enigo::new(&Settings::default()).map_err(|e| {
    format!(
      "enigo init failed: {e:?}\n\
      On macOS you must enable **System Settings → Privacy & Security \
      → Accessibility → allow Terminal or your app**"
    )
  })?;

  let _ = enigo.key(Key::Meta, Press);

  #[cfg(not(target_os = "macos"))]
  let _ = enigo.key(Key::Control, Press);

  let _ = enigo.key(Key::Unicode('v'), Click);

  #[cfg(target_os = "macos")]
  let _ = enigo.key(Key::Meta, Release);

  #[cfg(not(target_os = "macos"))]
  let _ = enigo.key(Key::Control, Release);

  Ok(())
}
