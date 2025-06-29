#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use reqwest::Client;
use text_fixer_lib::*;
#[cfg(not(target_os = "macos"))]
// use tokio::io::ReadHalf; // import from lib.rs
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

#[derive(Default)]
pub struct PastePlugin {
  last_window: Arc<Mutex<Option<x_win::WindowInfo>>>,
}

fn main() {
  // Load variables from `.env` at project root so OPENAI_API_KEY is available
  dotenvy::dotenv().ok();
  tauri::Builder::default()
    .plugin(tauri_plugin_clipboard_manager::init())
    .manage(PastePlugin::default())
    .invoke_handler(tauri::generate_handler![
      hide_window,
      paste_into_previous_app,
      get_last_window_name,
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
                    if let Some(w) = get_last_window(&plugin_state) {
                      // TODO: this is the wrong way to center the application window
                      let x = (((w.position.x + w.position.width) as f32) / 2.) - 150.;
                      let y = (((w.position.y + w.position.height) as f32) / 2.) - 22.;
                      let _ = window.set_position(tauri::LogicalPosition::new(x, y));
                    }
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

use regex::Regex;
/// Build a prompt that rewrites `text` into clear, everyday language and calls the LLM.
/// Returns the rewritten string or an error string.
async fn improve(text: &str) -> Result<String, String> {
  // Load prompt from file
  let prompt_template = match std::fs::read_to_string("../prompt.txt") {
    Ok(template) => template,
    Err(e) => {
      return Err(format!("Failed to read prompt file: {}", e));
    }
  };

  let prompt = prompt_template.replace("{text}", text.trim());
  let result = get_openai_response(&prompt).await?;
  let re = Regex::new(r#"<([^>]+)>"#).unwrap();

  let extracted = re
    .captures(&result)
    .and_then(|caps| caps.get(1))
    .map(|m| m.as_str().to_string())
    .unwrap_or_else(|| "No response".to_string());

  Ok(extracted + " ")
}

#[tauri::command]
fn hide_window(app: AppHandle) {
  if let Some(window) = app.get_webview_window("main") {
    let _ = window.hide();
  }
}

fn get_last_window(state: &tauri::State<'_, PastePlugin>) -> Option<x_win::WindowInfo> {
  let guard = state.last_window.lock().unwrap();
  guard.clone()
}

#[tauri::command]
async fn get_last_window_name(state: tauri::State<'_, PastePlugin>) -> Result<String, String> {
  let last_window = get_last_window(&state);

  match last_window {
    Some(win) if !win.info.name.trim().is_empty() => Ok(win.info.name.clone()),
    Some(_) => Ok("No title".to_string()),
    None => Ok("No previous window".to_string()),
  }
}

#[tauri::command]
async fn paste_into_previous_app(
  text: String,
  app: tauri::AppHandle,
  state: tauri::State<'_, PastePlugin>,
) -> Result<(String, String), String> {
  let response = improve(&text).await?;

  let last_window = get_last_window(&state);

  if let Some(_win) = last_window {
    // window_ops::activate(&win).map_err(|e| e.to_string())?;
    hide_window(app);
    platform_paste_text(&response)?;
  }
  Ok((text, response))
}

#[allow(dead_code)]
fn platform_send_paste_shortcut(_: &x_win::WindowInfo) -> Result<(), String> {
  Ok(())
}

use rdev::{simulate, EventType, Key, SimulateError};
use std::{thread, time};
fn send(event_type: &EventType) {
  // TODO: have to figure out how low this delay can be
  let delay = time::Duration::from_millis(20);
  match simulate(event_type) {
    Ok(()) => (),
    Err(SimulateError) => {
      println!("We could not send {:?}", event_type);
    }
  }
  // Let ths OS catchup (at least MacOS)
  thread::sleep(delay);
}

/// Copy `text` to the system clipboard and simulate Cmd/Ctrl‑V so it
/// appears in the now‑focused window.
fn platform_paste_text(text: &str) -> Result<(), String> {
  let mut cb = Clipboard::new().map_err(|e| e.to_string())?;
  cb.set_text(text.to_owned()).map_err(|e| e.to_string())?;

  send(&EventType::KeyPress(Key::ControlLeft));
  send(&EventType::KeyPress(Key::KeyV));
  send(&EventType::KeyRelease(Key::ControlLeft));
  Ok(())
}
